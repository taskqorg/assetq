"""Build a checksum-verified release candidate and run its selftest (A8)."""
from pathlib import Path
import hashlib
import json
import platform
import shutil
import subprocess
import sys
import tarfile
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def main():
    version = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]
    host = subprocess.run(["rustc", "-vV"], check=True, text=True, capture_output=True).stdout
    target = next(line.split()[1] for line in host.splitlines() if line.startswith("host:"))
    subprocess.run(["cargo", "build", "--release", "--locked"], cwd=ROOT, check=True, timeout=900)
    revision = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, check=True, text=True, capture_output=True).stdout.strip()
    if subprocess.run(["git", "status", "--porcelain", "--untracked-files=normal"], cwd=ROOT, check=True, text=True, capture_output=True).stdout.strip():
        raise RuntimeError("candidate requires a clean committed source revision")
    name = f"assetq-{version}-{revision[:12]}-{target}"
    dist = ROOT / "dist"
    stage = dist / name
    shutil.rmtree(stage, ignore_errors=True)
    (stage / "bin").mkdir(parents=True)
    executable = "assetq.exe" if sys.platform == "win32" else "assetq"
    shutil.copy2(ROOT / "target/release" / executable, stage / "bin" / executable)
    for doc in ["assetq.md", "docs/service.md", "docs/method-evaluation.md", "docs/deploy.md", "docs/architecture.md", "docs/acceptance.md", "docs/meshy-baseline.md", "docs/meshy-pipeline.md", "docs/evidence/5/operator.md"]:
        (stage / doc).parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(ROOT / doc, stage / doc)
    binary_sha = hashlib.sha256((stage / "bin" / executable).read_bytes()).hexdigest()
    (stage / "SHA256SUMS").write_text(f"{binary_sha}  bin/{executable}\n")
    manifest = {"source_sha": revision, "target": target, "version": version, "schema": 1,
                "binary_sha256": binary_sha, "libc": platform.libc_ver(), "synthetic_provider_only": True,
                "embedded_ui": {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted((ROOT / "web").iterdir()) if p.is_file()}}
    (stage / "candidate.json").write_text(json.dumps(manifest, indent=2) + "\n")
    archive = dist / f"{name}.tar.gz"
    with tarfile.open(archive, "w:gz") as tar:
        tar.add(stage, arcname=name)
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    (dist / "SHA256SUMS").write_text(f"{digest}  {archive.name}\n")
    shutil.rmtree(stage)
    # Verify the packaged bytes and run the extracted executable, not the build tree.
    assert hashlib.sha256(archive.read_bytes()).hexdigest() == digest
    with tempfile.TemporaryDirectory(prefix="aq-candidate-") as directory:
        with tarfile.open(archive) as tar:
            tar.extractall(directory, filter="data")
        binary = Path(directory) / name / "bin" / executable
        assert hashlib.sha256(binary.read_bytes()).hexdigest() == binary_sha
        subprocess.run([str(binary), "selftest"], check=True, timeout=60)
    print(json.dumps(manifest, sort_keys=True))
    print(f"{digest}  {archive}")


if __name__ == "__main__":
    try:
        main()
    except (subprocess.SubprocessError, OSError, RuntimeError) as error:
        print(error, file=sys.stderr)
        raise SystemExit(1)
