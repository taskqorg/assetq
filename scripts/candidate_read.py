"""Build the checksum-verified pipe-read release; apart from the Pipe candidate because it deploys apart."""
from pathlib import Path
import hashlib
import shutil
import subprocess
import tarfile
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def main():
    version = tomllib.loads((ROOT / "lane/Cargo.toml").read_text())["package"]["version"]
    host = subprocess.run(["rustc", "-vV"], check=True, text=True, capture_output=True).stdout
    target = next(line.split()[1] for line in host.splitlines() if line.startswith("host:"))
    subprocess.run(["cargo", "build", "--release", "--locked", "--manifest-path", "lane/Cargo.toml"], cwd=ROOT, check=True, timeout=900)
    revision = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, check=True, text=True, capture_output=True).stdout.strip()
    name = f"pipe-read-{version}-{revision[:12]}-{target}"
    dist = ROOT / "dist-read"
    shutil.rmtree(dist, ignore_errors=True)
    stage = dist / name
    (stage / "bin").mkdir(parents=True)
    shutil.copy2(ROOT / "lane/target/release/pipe-read", stage / "bin/pipe-read")
    shutil.copytree(ROOT / "deploy/read-lane", stage / "deploy/read-lane")
    for script in ["read_lane_backup.sh", "read_lane_restore.sh", "read_lane_probe.py", "read_lane_alert.sh", "pipe_mcp.py"]:
        (stage / "scripts").mkdir(exist_ok=True)
        shutil.copy2(ROOT / "scripts" / script, stage / "scripts" / script)
    archive = dist / f"{name}.tar.gz"
    with tarfile.open(archive, "w:gz") as tar:
        tar.add(stage, arcname=name)
    shutil.rmtree(stage)
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    (dist / "SHA256SUMS").write_text(f"{digest}  {archive.name}\n")
    # Run the packaged bytes, not the build tree.
    with tempfile.TemporaryDirectory(prefix="pipe-read-candidate-") as directory:
        with tarfile.open(archive) as tar:
            tar.extractall(directory, filter="data")
        subprocess.run([str(Path(directory) / name / "bin/pipe-read"), "selftest"], check=True, timeout=30)
    print(archive)


if __name__ == "__main__":
    main()
