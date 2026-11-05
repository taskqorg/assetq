"""Retained image qualification helpers; new qualification is refused under Pipe R10."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import time
import urllib.error
import urllib.request
import zlib


TEXT = ("One original stylized blue wooden wedge on a plain background, no text or third-party references. "
        "Use the attached invented blue swatch as a color reference. Generate exactly one PNG up to 1024 by 1024 pixels.")


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
    parser.parse_args()
    if "TASKQ_ATTEMPT" in os.environ:
        raise RuntimeError("worker cannot control external slots/app servers; use the existing operator channel, never remove role variables")
    raise RuntimeError("legacy TaskQ adapter is recovery-only; new qualification requires independent AgentQ/CoreQ authority")


if __name__ == "__main__":
    main()
