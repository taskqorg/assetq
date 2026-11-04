"""Trusted headless Blender validator; never rewrites the source GLB."""
import json
import math
from pathlib import Path
import sys


def inspect_scene(bpy):
    meshes = [obj for obj in bpy.context.scene.objects if obj.type == "MESH"]
    if not meshes or len(meshes) > 64:
        raise ValueError("expected 1..64 mesh objects")
    triangles = 0
    components = 0
    uv_faces = 0
    total_faces = 0
    points = []
    for obj in meshes:
        mesh = obj.data
        mesh.calc_loop_triangles()
        triangles += len(mesh.loop_triangles)
        total_faces += len(mesh.polygons)
        if mesh.uv_layers.active:
            uv_faces += len(mesh.polygons)
        parents = list(range(len(mesh.vertices)))

        def root(index):
            while parents[index] != index:
                parents[index] = parents[parents[index]]
                index = parents[index]
            return index

        for edge in mesh.edges:
            left, right = edge.vertices
            parents[root(left)] = root(right)
        used = {index for face in mesh.polygons for index in face.vertices}
        components += len({root(index) for index in used})
        points.extend(tuple(obj.matrix_world @ vertex.co) for vertex in mesh.vertices)
    if not points or any(not math.isfinite(v) for point in points for v in point):
        raise ValueError("empty or nonfinite geometry")
    dimensions = [max(p[axis] for p in points) - min(p[axis] for p in points) for axis in range(3)]
    textures = []
    for image in bpy.data.images:
        if image.type != "IMAGE":
            continue
        textures.append({"name": image.name, "width": image.size[0], "height": image.size[1],
                         "channels": image.channels, "packed": bool(image.packed_file)})
    materials = []
    mapped = set()
    for material in bpy.data.materials:
        if not material.use_nodes:
            continue
        roles = []
        for node in material.node_tree.nodes:
            if node.type != "BSDF_PRINCIPLED":
                continue
            for name in ("Base Color", "Metallic", "Roughness", "Normal"):
                socket = node.inputs.get(name)
                if socket and socket.is_linked:
                    roles.append(name)
                    mapped.add(name)
        materials.append({"name": material.name, "linked_pbr_inputs": roles})
    failures = []
    if not 0 < triangles <= 100000:
        failures.append("triangle safety bound 1..100000 exceeded")
    if uv_faces != total_faces:
        failures.append("some faces have no UV layer")
    if not textures or any(t["width"] != 2048 or t["height"] != 2048 for t in textures):
        failures.append("embedded textures do not match the 2048-by-2048 recipe")
    if not {"Base Color", "Metallic", "Roughness", "Normal"}.issubset(mapped):
        failures.append("missing connected PBR material inputs")
    if any(v <= 0 for v in dimensions):
        failures.append("degenerate bounding dimensions")
    return {"valid": not failures, "reason": "; ".join(failures), "blender_version": bpy.app.version_string,
            "triangles": triangles, "target_triangles": 3000, "triangle_delta": triangles - 3000,
            "components": components, "mesh_objects": len(meshes), "dimensions": dimensions,
            "dimension_units": "provider GLB units; no real-world scale inferred", "uv_faces": uv_faces,
            "faces": total_faces, "textures": textures, "materials": materials}


def render_views(bpy, report, directory):
    from mathutils import Vector
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.samples = 16
    scene.cycles.device = "CPU"
    scene.render.threads_mode = "FIXED"
    scene.render.threads = 2
    scene.render.resolution_x = 512
    scene.render.resolution_y = 512
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.world.color = (0.25, 0.25, 0.25)
    objects = [obj for obj in scene.objects if obj.type == "MESH"]
    points = [obj.matrix_world @ vertex.co for obj in objects for vertex in obj.data.vertices]
    low = Vector([min(p[axis] for p in points) for axis in range(3)])
    high = Vector([max(p[axis] for p in points) for axis in range(3)])
    center = (low + high) / 2
    radius = max(high - low)
    bpy.ops.object.light_add(type="AREA", location=center + Vector((radius, -radius, radius * 2)))
    light = bpy.context.object
    light.data.energy = 500 * radius * radius
    light.data.shape = "DISK"
    light.data.size = radius * 3
    light.rotation_euler = (center - light.location).to_track_quat("-Z", "Y").to_euler()
    bpy.ops.object.camera_add()
    camera = bpy.context.object
    scene.camera = camera
    camera.data.type = "ORTHO"
    camera.data.ortho_scale = radius * 1.8
    camera.data.clip_start = max(radius / 10000, 0.00001)
    camera.data.clip_end = radius * 100
    report["views"] = []
    for name, direction in [("front", (1, -1.5, 1)), ("back", (-1, 1.5, 1)), ("side", (1.5, 1, 0.5))]:
        camera.location = center + Vector(direction) * radius * 3
        camera.rotation_euler = (center - camera.location).to_track_quat("-Z", "Y").to_euler()
        filename = f"view-{name}.png"
        scene.render.filepath = str(directory / filename)
        bpy.ops.render.render(write_still=True)
        if (directory / filename).stat().st_size > 4 << 20:
            raise ValueError("render exceeds 4 MiB")
        report["views"].append(filename)


def main():
    import bpy
    glb, output = sys.argv[sys.argv.index("--") + 1:]
    directory = Path(output)
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    try:
        bpy.ops.import_scene.gltf(filepath=glb)
        report = inspect_scene(bpy)
        report["original_texture_maps"] = []
        for item in json.loads((directory / "inputs.json").read_text(encoding="utf-8")):
            if not item["role"].startswith("texture-"):
                continue
            texture = bpy.data.images.load(item["path"], check_existing=True)
            report["original_texture_maps"].append({"role": item["role"], "sha256": item["sha256"],
                                                    "width": texture.size[0], "height": texture.size[1],
                                                    "format": texture.file_format})
            if tuple(texture.size) != (2048, 2048):
                report["valid"] = False
                report["reason"] += "; original texture map does not match the 2048-by-2048 recipe"
        if report["valid"]:
            render_views(bpy, report, directory)
    except Exception as error:
        report = {"valid": False, "reason": f"Blender validation refused: {type(error).__name__}",
                  "blender_version": bpy.app.version_string, "views": []}
    temporary = directory / "validation.pending"
    temporary.write_text(json.dumps(report, indent=2), encoding="utf-8")
    temporary.replace(directory / "validation.json")


if __name__ == "__main__":
    main()
