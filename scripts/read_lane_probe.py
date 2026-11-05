#!/usr/bin/env python3
"""Off-host read lane probe: one check per run, alerting once per transition (pipe.md § Read lane).

usage: read_lane_probe.py --url URL --state FILE --alert COMMAND [--access-file FILE]
The alert command is the host's existing alert path; it receives one line as its last argument.
"""
import argparse
import json
from pathlib import Path
import shlex
import socket
import subprocess
import sys
import urllib.error
import urllib.request


def access_headers(path):
    """Guard headers from a shell env file, read at use so that its values are never copied."""
    names = {"CF_ACCESS_CLIENT_ID": "CF-Access-Client-Id", "CF_ACCESS_CLIENT_SECRET": "CF-Access-Client-Secret"}
    values = dict(line.removeprefix("export ").strip().split("=", 1) for line in Path(path).read_text().splitlines() if "=" in line)
    return {header: values[name].strip("'\"") for name, header in names.items()}


def observe(url, headers):
    """Returns None when healthy, else the reason. A probe failure is not proof the lane is down, so it says what it saw."""
    try:
        with urllib.request.urlopen(urllib.request.Request(f"{url}/healthz", headers=headers), timeout=20) as reply:
            status = json.load(reply).get("status")
    except urllib.error.HTTPError as error:
        return f"health answered HTTP {error.code}"
    except (OSError, ValueError) as error:
        return f"health unreachable from this probe: {error}"
    return None if status == "ok" else f"health status {status}"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--url", required=True)
    parser.add_argument("--state", required=True, type=Path)
    parser.add_argument("--alert", required=True)
    parser.add_argument("--access-file", type=Path)
    options = parser.parse_args()
    headers = access_headers(options.access_file) if options.access_file else {}
    reason = observe(options.url.rstrip("/"), headers)
    previous = options.state.read_text() if options.state.exists() else ""
    host = socket.gethostname()
    message = None
    if reason and previous != "failing":
        message = f"pipe-read probe on {host}: {reason}"
    elif not reason and previous == "failing":
        message = f"pipe-read probe on {host}: Alert cleared"
    sent = True
    if message:
        sent = subprocess.run([*shlex.split(options.alert), message]).returncode == 0
    # An alert that did not go out is retried on the next run.
    if sent:
        options.state.write_text("failing" if reason else "ok")
    print(message or reason or "ok")
    sys.exit(1 if reason else 0)


if __name__ == "__main__":
    main()
