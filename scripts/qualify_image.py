"""One approved local image job; operator context only. Never rerun its directory."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import selectors
import struct
import subprocess
import tempfile
import time
import urllib.error
import urllib.request
import zlib

from check import pg_bin, run

TEXT = ("One original stylized blue wooden wedge on a plain background, no text or third-party references. "
        "Use the attached invented blue swatch as a color reference. Generate exactly one PNG up to 1024 by 1024 pixels.")
ADAPTER = "bd6dc0de5024ff7b6b0ae0a987f5467aa0c5196b005c7cdfd434d043709c7e42"


def swatch():
    # Invented input only. Successful output must be the native tool's original bitmap.
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 64, 64, 8, 2, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress((b"\0" + bytes([40, 100, 200]) * 64) * 64)) + chunk(b"IEND", b""))


def save(directory, name, value):
    path = directory / name
    temporary = directory / (name + ".tmp")
    with temporary.open("w") as f:
        json.dump(value, f, indent=2)
        f.flush()
        os.fsync(f.fileno())
    temporary.replace(path)


def request(url, token, path, body=None, content_type="application/json"):
    headers = {"Authorization": "Bearer " + token, "Content-Type": content_type}
    data = body if isinstance(body, bytes) else json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(url + path, data=data, headers=headers)
    with urllib.request.urlopen(req, timeout=20) as response:
        data = response.read(65 * 1024 * 1024)
        if content_type == "download":
            return data, response.headers["X-Content-Sha256"]
        return json.loads(data)


def exercise(api, restart, directory, attachment, wait=time.sleep, clock=time.monotonic):
    body = {"method": "image.worker.v1@5", "operation": "image.generate", "text": TEXT,
            "attachments": [attachment], "constraints": {"max_width": 1024, "max_height": 1024},
            "budget": {"max_credits": 0}, "idempotency_key": directory.name}
    first = api("/v1/jobs", body)
    assert first["created"] is True
    job = first["job"]["id"]
    save(directory, "accepted.json", first)
    duplicate = api("/v1/jobs", body)
    assert duplicate["created"] is False and duplicate["job"]["id"] == job
    save(directory, "duplicate-before.json", duplicate)
    restart(job)  # Replace admission-only control service with the explicitly job-bound service.
    deadline = clock() + 720
    interrupted = False
    operation = None
    while clock() < deadline:
        status = api("/v1/jobs/" + job)
        save(directory, "status.json", status)
        save(directory, "events.json", api("/v1/jobs/" + job + "/events"))
        spool = directory / "store/tmp/image-worker" / str(status.get("provider_operation"))
        # Interrupt control only after input intent; leave the detached original runner intact.
        if not interrupted and status.get("provider_operation") and (spool / "native.jsonl").exists():
            if '"operation":"input"' in (spool / "native.jsonl").read_text():
                operation = status["provider_operation"]
                restart(job)
                duplicate = api("/v1/jobs", body)
                assert duplicate["created"] is False and duplicate["job"]["id"] == job
                save(directory, "duplicate-after.json", duplicate)
                interrupted = True
        if status["phase"] in ("succeeded", "failed", "cancelled"):
            break
        wait(1)
    else:
        save(directory, "cancel-request.json", api("/v1/jobs/" + job + "/cancel", {}))
        raise RuntimeError("qualification deadline; original operation/reservations retained; no replay")
    assert status["submissions"] == 1
    if operation:
        assert status["provider_operation"] == operation
    if status["phase"] != "succeeded":
        raise RuntimeError("native job did not succeed; inspect retained refusal/unknown evidence; no second turn")
    assert interrupted, "control interruption was not exercised; keep this acceptance gap open"
    result = api("/v1/jobs/" + job + "/result")
    save(directory, "result.json", result)
    items = result["manifest"]["items"]
    assert len(items) == 3
    for item in items:
        data, header = api("/v1/objects/" + item["object_id"] + "/content", download=True)
        digest = hashlib.sha256(data).hexdigest()
        assert digest == item["sha256"] == header
        (directory / (item["import_key"] + ".download")).write_bytes(data)
        save(directory, item["import_key"] + ".json", api("/v1/objects/" + item["object_id"]))
    return job


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--adapter", type=Path, required=True)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--directory", type=Path, required=True)
    args = parser.parse_args()
    if "TASKQ_ATTEMPT" in os.environ:
        raise RuntimeError("worker cannot control external slots/app servers; use the existing operator channel, never remove role variables")
    workspace, binary, adapter = (p.resolve(strict=True) for p in (args.workspace, args.binary, args.adapter))
    assert hashlib.sha256(adapter.read_bytes()).hexdigest() == ADAPTER
    directory = args.directory.absolute()
    assert directory.parent.resolve(strict=True).is_relative_to(workspace)
    directory.mkdir(mode=0o700)  # Create-once run identity; do not replace or clean it.
    os.umask(0o077)
    binaries, cluster = pg_bin(), directory / "postgres"
    socket_dir = Path(tempfile.mkdtemp(prefix="aqi-", dir="/tmp"))
    env = dict(os.environ, LC_ALL="C", LANG="C", ASSETQ_DATA_ROOT=str(directory / "store"),
               ASSETQ_IMAGE_WORKER_ROOT=str(workspace), ASSETQ_EXECUTORS="0", ASSETQ_MAX_EXECUTION_MS="600000",
               ASSETQ_POLL_MS="500", ASSETQ_QUOTA_BYTES=str(1 << 30), ASSETQ_TEMP_QUOTA_BYTES=str(1 << 30),
               ASSETQ_MAX_UPLOAD_BYTES=str(64 << 20))
    if "ASSETQ_FAKE_PROVIDER_DIR" in env:
        raise RuntimeError("synthetic provider configuration present; qualification refuses")
    run(str(binaries / "initdb"), "-D", str(cluster), "-U", "assetq", "--auth=trust", "-E", "UTF8", "--no-locale", env=env)
    # No TCP listener; private isolated socket and durable fsync remain enabled.
    run(str(binaries / "pg_ctl"), "-D", str(cluster), "-l", str(directory / "postgres.log"), "-w",
        "-o", f"-k {socket_dir} -c listen_addresses=''", "start", env=env)
    env["ASSETQ_DATABASE_URL"] = f"host={socket_dir} user=assetq dbname=postgres"
    save(directory, "local-database.json", {"socket": str(socket_dir), "cluster": str(cluster)})
    child = None
    logs = []
    token = None
    url = None
    try:
        def cli(*parts):
            return run(str(binary), *parts, cwd=workspace, env=env).strip()
        cli("migrate")
        cli("project", "image-qualification-4", "--budget", "0", "--max-active", "1")
        token = cli("token", "image-qualification-4")
        # Token and process environment remain private and are never printed or saved.
        save(directory, "candidate.json", {"binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
             "adapter_sha256": ADAPTER, "usage": "unknown until native report; unreported subscription charge remains unknown"})
        def restart(job=None):
            nonlocal child, url
            if child is not None:
                child.kill()
                child.wait(timeout=10)
            stderr = (directory / f"service-{len(logs)}.stderr").open("wb")
            logs.append(stderr)
            settings = dict(env, ASSETQ_EXECUTORS="1" if job else "0")
            command = [str(binary), "image-serve", job, str(adapter)] if job else [str(binary), "serve", "--listen", "127.0.0.1:0"]
            child = subprocess.Popen(command, cwd=workspace, env=settings, stdout=subprocess.PIPE, stderr=stderr)
            with selectors.DefaultSelector() as ready:
                ready.register(child.stdout, selectors.EVENT_READ)
                if not ready.select(15):
                    raise RuntimeError("isolated service startup unobserved; inspect private stderr")
            line = child.stdout.readline().decode().strip()
            if not line.startswith("assetq listening on 127.0.0.1:"):
                raise RuntimeError("isolated service refused startup; inspect private stderr")
            url = "http://" + line.removeprefix("assetq listening on ")
        restart()
        def api(path, body=None, download=False):
            return request(url, token, path, body, "download" if download else "application/json")
        uploaded = request(url, token, "/v1/uploads", swatch(), "image/png")
        save(directory, "attachment.json", uploaded)
        assert uploaded["sha256"] == hashlib.sha256(swatch()).hexdigest()
        # Verified ID is accepted by the durable API before the scoped command exists.
        job = exercise(api, restart, directory, uploaded["id"])
        save(directory, "audit.json", json.loads(cli("audit", job)))
        print(json.dumps({"state": "retained", "job": job, "visual_acceptance": "pending", "charge": "unknown unless reported"}))
    finally:
        if child is not None and child.poll() is None:
            child.kill()
            child.wait(timeout=10)
        for log in logs:
            log.close()
        # Stop only this disposable metadata cluster. Its data and original spool remain for recovery.
        run(str(binaries / "pg_ctl"), "-D", str(cluster), "-m", "fast", "stop", env=env)
        socket_dir.rmdir()


if __name__ == "__main__":
    main()
