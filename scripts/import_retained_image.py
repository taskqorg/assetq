"""Import the approved retained PNG from the original local qualification; no generation."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import selectors
import subprocess
import tempfile
import time

from check import pg_bin, run
from qualify_image import request, save


def import_request(source):
    job = source["id"]
    return {"method": "image.worker.v1@6", "operation": "image.import_native", "text": source["request"]["text"],
            "attachments": source["request"]["attachments"], "constraints": {"source_job": job},
            "budget": {"max_credits": 0}, "idempotency_key": "native-size-import-" + job}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("binary", "workspace", "directory"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--sha256", required=True)
    args = parser.parse_args()
    if "TASKQ_ATTEMPT" in os.environ:
        raise RuntimeError("worker cannot reconcile external app servers; use existing operator channel, never remove role variables")
    binary, workspace, directory = (p.resolve(strict=True) for p in (args.binary, args.workspace, args.directory))
    assert directory.is_relative_to(workspace)
    assert len(args.sha256) == 64 and all(c in "0123456789abcdef" for c in args.sha256)
    source = json.loads((directory / "accepted.json").read_text())["job"]["id"]
    cluster = directory / "postgres"
    assert cluster.is_dir() and (directory / "store").is_dir()
    evidence = directory / "native-size-import"
    evidence.mkdir(mode=0o700, exist_ok=True)  # Same idempotency key and retained DB on recovery, never a new turn.
    os.umask(0o077)
    socket_dir = Path(tempfile.mkdtemp(prefix="aqr-", dir="/tmp"))
    binaries = pg_bin()
    env = dict(os.environ, LC_ALL="C", LANG="C", ASSETQ_DATA_ROOT=str(directory / "store"),
               ASSETQ_IMAGE_WORKER_ROOT=str(workspace), ASSETQ_EXECUTORS="1", ASSETQ_MAX_EXECUTION_MS="600000",
               ASSETQ_POLL_MS="500", ASSETQ_QUOTA_BYTES=str(1 << 30), ASSETQ_TEMP_QUOTA_BYTES=str(1 << 30),
               ASSETQ_MAX_UPLOAD_BYTES=str(64 << 20), ASSETQ_DATABASE_URL=f"host={socket_dir} user=assetq dbname=postgres")
    if "ASSETQ_FAKE_PROVIDER_DIR" in env:
        raise RuntimeError("synthetic provider configuration present; retained import refuses")
    run(str(binaries / "pg_ctl"), "-D", str(cluster), "-l", str(evidence / "postgres.log"), "-w",
        "-o", f"-k {socket_dir} -c listen_addresses=''", "start", env=env)
    child = None
    try:
        def cli(*parts):
            return run(str(binary), *parts, cwd=workspace, env=env).strip()
        # Only this isolated API credential is created; native grants/accounts stay unchanged.
        token = cli("token", "image-qualification-4")
        with (evidence / "service.stderr").open("ab") as stderr:
            child = subprocess.Popen([str(binary), "image-import-serve", source, args.sha256], cwd=workspace, env=env,
                                     stdout=subprocess.PIPE, stderr=stderr)
            with selectors.DefaultSelector() as ready:
                ready.register(child.stdout, selectors.EVENT_READ)
                if not ready.select(40):
                    raise RuntimeError("retained import startup unobserved; inspect private stderr; no new generation")
            line = child.stdout.readline().decode().strip()
            assert line.startswith("pipe listening on 127.0.0.1:"), "retained import startup refused; inspect private stderr"
            url = "http://" + line.removeprefix("pipe listening on ")
            def api(path, body=None):
                return request(url, token, path, body)
            deadline = time.monotonic() + 60
            while time.monotonic() < deadline:
                old = api("/v1/jobs/" + source)
                if old["phase"] == "failed":
                    break
                time.sleep(0.2)
            assert old["phase"] == "failed" and "PNG dimensions" in old["reason"]
            assert old["submissions"] == 1 and old["method"] == "image.worker.v1@5"
            save(evidence, "source-job.json", old)
            body = import_request(old)
            created = api("/v1/jobs", body)
            job = created["job"]["id"]
            duplicate = api("/v1/jobs", body)
            assert duplicate["created"] is False and duplicate["job"]["id"] == job
            save(evidence, "accepted.json", created)
            save(evidence, "duplicate.json", duplicate)
            while time.monotonic() < deadline:
                status = api("/v1/jobs/" + job)
                if status["phase"] in ("succeeded", "failed", "cancelled"):
                    break
                time.sleep(0.2)
            save(evidence, "status.json", status)
            assert status["phase"] == "succeeded", "retained import incomplete; keep original identity, never generate a replacement"
            result = api("/v1/jobs/" + job + "/result")
            save(evidence, "result.json", result)
            for item in result["manifest"]["items"]:
                data, header = request(url, token, "/v1/objects/" + item["object_id"] + "/content", content_type="download")
                digest = hashlib.sha256(data).hexdigest()
                assert digest == header == item["sha256"]
                if item["import_key"] == "output-0":
                    assert digest == args.sha256, "retained original differs from approved PNG"
                (evidence / (item["import_key"] + ".download")).write_bytes(data)
                save(evidence, item["import_key"] + ".json", api("/v1/objects/" + item["object_id"]))
            for identifier in (source, job):
                save(evidence, identifier + "-events.json", api("/v1/jobs/" + identifier + "/events"))
                save(evidence, identifier + "-audit.json", json.loads(cli("audit", identifier)))
            save(evidence, "candidate.json", {"binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "new_runtime_inputs": 0})
            print(json.dumps({"source_job": source, "import_job": job, "manifest": result["manifest"], "charge": "unknown when unreported"}))
    finally:
        if child is not None and child.poll() is None:
            child.kill()
            child.wait(timeout=10)
        run(str(binaries / "pg_ctl"), "-D", str(cluster), "-m", "fast", "stop", env=env)
        socket_dir.rmdir()


if __name__ == "__main__":
    main()
