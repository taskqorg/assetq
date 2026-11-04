"""Deterministic measurement checks; no native Blender qualification is implied."""
from types import SimpleNamespace as N
import unittest

from validate_meshy import inspect_scene


class Identity:
    def __matmul__(self, coordinate):
        return coordinate


def scene():
    vertices = [N(co=p) for p in [(0, 0, 0), (1, 0, 0), (0, 1, 0), (3, 0, 1), (4, 0, 1), (3, 1, 1)]]
    mesh = N(vertices=vertices, edges=[N(vertices=e) for e in [(0, 1), (1, 2), (2, 0), (3, 4), (4, 5), (5, 3)]],
             polygons=[N(vertices=(0, 1, 2)), N(vertices=(3, 4, 5))], loop_triangles=[None, None],
             uv_layers=N(active=True), calc_loop_triangles=lambda: None)
    inputs = {name: N(is_linked=True) for name in ["Base Color", "Metallic", "Roughness", "Normal"]}
    return N(context=N(scene=N(objects=[N(type="MESH", data=mesh, matrix_world=Identity())])),
             data=N(images=[N(type="IMAGE", name="invented.png", size=(2048, 2048), channels=4, packed_file=True)],
                    materials=[N(name="invented", use_nodes=True, node_tree=N(nodes=[N(type="BSDF_PRINCIPLED", inputs=inputs)]))]),
             app=N(version_string="invented fixture; not native Blender"))


class Measurements(unittest.TestCase):
    def test_disconnected_world_geometry_and_pbr(self):
        result = inspect_scene(scene())
        self.assertTrue(result["valid"])
        self.assertEqual((result["triangles"], result["components"], result["dimensions"]), (2, 2, [4, 1, 1]))
        self.assertEqual(result["triangle_delta"], -2998)

    def test_missing_uv_wrong_texture_and_nonfinite_geometry(self):
        bpy = scene()
        bpy.context.scene.objects[0].data.uv_layers.active = False
        bpy.data.images[0].size = (1024, 2048)
        result = inspect_scene(bpy)
        self.assertFalse(result["valid"])
        self.assertIn("UV", result["reason"])
        self.assertIn("2048", result["reason"])
        bpy.context.scene.objects[0].data.vertices[0].co = (float("nan"), 0, 0)
        with self.assertRaisesRegex(ValueError, "nonfinite"):
            inspect_scene(bpy)


if __name__ == "__main__":
    unittest.main()
