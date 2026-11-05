#!/usr/bin/env python3
"""Plan, apply and roll back one verified AssetQ CI candidate over existing operator SSH.

`plan` and `apply --dry-run` only read GitHub and the host. `apply` and `rollback` are
LIVE: they require an interactive operator, the reviewed plan digest and a typed phrase
per action. See docs/deployment-operator.md. This tool has no DNS, proxy, access, grant,
credential, unit-editing or destructive-storage path."""
import argparse
import base64
import datetime
import hashlib
import io
import json
import os
import re
import shlex
import subprocess
import sys
import tarfile
import time
import zipfile
from pathlib import Path

# Compiled in, never read from a plan, target or environment (CoreQ R8).
REPO = "taskqorg/assetq"
WORKFLOW = ".github/workflows/tests.yml"
ARTIFACT = "assetq-candidate"
REQUIRED_JOBS = ("tests", "candidate")
TARGET = "x86_64-unknown-linux-gnu"
FORMAT = "assetq.deploy-plan/1"
PLAN_TTL_SECONDS = 3600
MAX_ARTIFACT_BYTES = 512 * 1024 * 1024
SERVICE_SECONDS = 300
SELFTEST_SECONDS = 60
HEALTH_INTERVAL_SECONDS = 2
SSH = ["ssh"]
GH = ["gh"]
ETC = "/etc"
PROC = "/proc"
HOST_TOOLS = ("id", "runuser", "env", "systemctl", "systemd-run", "sha256sum", "tar", "curl", "readlink", "ln", "mv", "cp",
              "chown", "df", "stat", "cut", "sed", "tr", "hostname", "getconf", "ssh-keygen")
MONOTONIC = ("jobs", "succeeded", "blobs", "methods", "attempts", "physical_object_bytes")
TERMINAL_PHASES = {"succeeded", "failed", "cancelled"}
UNIT_PROPS = ("ExecStart", "EnvironmentFiles", "ActiveState", "MainPID", "LimitAS", "LimitSTACK",
              "AllowedCPUs", "MemoryMax", "TasksMax")
INSPECT_KEYS = ("hostname", "machine_id_sha256", "ssh_host_key_sha256", "libc", "current", "previous", "current_binary_sha256",
                "unit_sha256", "install_free", "data_free", "data_root_stat")
LIMIT_PROPS = ("LimitAS", "LimitSTACK", "AllowedCPUs", "MemoryMax", "TasksMax")
EXIT_REFUSED, EXIT_ROLLED_BACK, EXIT_UNCERTAIN = 1, 2, 3


class Refused(Exception):
    """Stopped before any further effect; the running service was not changed by this step."""


class Uncertain(Exception):
    """A host outcome is unknown after an effect; the lock stays and nothing is retried (R4)."""


class Transport(Exception):
    """SSH itself failed or timed out, so the remote outcome was not observed."""


def now():
    return datetime.datetime.now(datetime.timezone.utc)


def stamp(moment):
    return moment.strftime("%Y-%m-%dT%H:%M:%SZ")


def sha256_hex(data):
    return hashlib.sha256(data).hexdigest()


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def git_blob_sha(data):
    return hashlib.sha1(b"blob %d\0" % len(data) + data).hexdigest()


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode()


def gh_bytes(path):
    try:
        result = subprocess.run(GH + ["api", path], capture_output=True, timeout=300)
    except (OSError, subprocess.TimeoutExpired) as error:
        raise Refused(f"GitHub evidence unavailable for {path}: {error}")
    if result.returncode:
        raise Refused(f"GitHub evidence unavailable for {path}: {result.stderr.decode(errors='replace').strip()[:300]}")
    return result.stdout


def gh_json(path):
    try:
        return json.loads(gh_bytes(path))
    except ValueError:
        raise Refused(f"GitHub evidence unreadable for {path}")


# ---------------------------------------------------------------- target


def load_target(path):
    try:
        target = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise Refused(f"target file unreadable: {error}")
    required = {"ssh_alias", "service_account", "service_uid", "executable", "install_root", "unit", "port", "data_root",
                "expected", "status_env_files",
                "min_install_free_bytes", "min_data_free_bytes", "health_timeout_seconds"}
    if not isinstance(target, dict) or set(target) - {"ssh_user"} != required:
        raise Refused(f"target file must have these fields: {sorted(required)}; optional: ssh_user")
    expected = target["expected"]
    if not isinstance(expected, dict) or set(expected) != {"hostname", "machine_id_sha256", "ssh_host_key_sha256"} \
            or not all(isinstance(v, str) and v for v in expected.values()):
        raise Refused("target expected identity must name hostname, machine_id_sha256 and ssh_host_key_sha256")
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]*", str(target["ssh_alias"])):
        raise Refused("target ssh_alias is not a plain SSH alias")
    if "ssh_user" in target and (not isinstance(target["ssh_user"], str)
                                 or not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_.-]*", target["ssh_user"])):
        raise Refused("target ssh_user is not a plain SSH username")
    if not re.fullmatch(r"[a-z0-9][a-z0-9@._-]*\.service", str(target["unit"])):
        raise Refused("target unit is not a service name")
    if not re.fullmatch(r"[a-z_][a-z0-9_-]*", str(target["service_account"])):
        raise Refused("target service_account is not an account name")
    if not re.fullmatch(r"[A-Za-z0-9_-]+(/[A-Za-z0-9._-]+)*", str(target["executable"])):
        raise Refused("target executable must be a plain path relative to a release")
    paths = [target["install_root"], target["data_root"]] + list(target["status_env_files"])
    if not all(isinstance(p, str) and re.fullmatch(r"(/[A-Za-z0-9._-]+)+", p) and "/../" not in f"{p}/" for p in paths):
        raise Refused("target paths must be plain absolute paths")
    for key, low, high in [("service_uid", 1, 2**31 - 1), ("port", 1024, 65535), ("min_install_free_bytes", 1, 2**62),
                           ("min_data_free_bytes", 1, 2**62), ("health_timeout_seconds", 4, 300)]:
        if not isinstance(target[key], int) or isinstance(target[key], bool) or not low <= target[key] <= high:
            raise Refused(f"target {key} must be an integer from {low} to {high}")
    return target


# ---------------------------------------------------------------- GitHub and candidate evidence


def collect_ci(source):
    """Green merge-result CI for the exact source, and the immutable artifact it produced."""
    if not re.fullmatch(r"[0-9a-f]{40}", source):
        raise Refused("source must be a full lowercase commit SHA")
    runs = gh_json(f"repos/{REPO}/actions/runs?head_sha={source}&event=merge_group&per_page=100").get("workflow_runs", [])
    runs = [r for r in runs if r.get("path") == WORKFLOW and r.get("head_sha") == source and r.get("event") == "merge_group"]
    if not runs:
        raise Refused(f"no merge-result CI run for {source}")
    for run in runs:
        if (run.get("status"), run.get("conclusion")) != ("completed", "success"):
            raise Refused(f"merge-result CI run {run.get('id')} is {run.get('status')}/{run.get('conclusion')}, not green")
    run = max(runs, key=lambda r: r["id"])
    jobs = gh_json(f"repos/{REPO}/actions/runs/{run['id']}/jobs?filter=latest&per_page=100").get("jobs", [])
    conclusions = {j.get("name"): j.get("conclusion") for j in jobs}
    for name in REQUIRED_JOBS:
        if conclusions.get(name) != "success":
            raise Refused(f"merge-result CI job {name} is {conclusions.get(name)}, not success")
    artifacts = gh_json(f"repos/{REPO}/actions/runs/{run['id']}/artifacts?per_page=100").get("artifacts", [])
    artifacts = [a for a in artifacts if a.get("name") == ARTIFACT]
    if len(artifacts) != 1:
        raise Refused(f"run {run['id']} has {len(artifacts)} {ARTIFACT} artifacts, expected one")
    artifact = artifacts[0]
    origin = artifact.get("workflow_run") or {}
    if artifact.get("expired") or origin.get("id") != run["id"] or origin.get("head_sha") != source:
        raise Refused(f"artifact {artifact.get('id')} is expired or not bound to run {run['id']} at {source}")
    digest = str(artifact.get("digest") or "")
    if not re.fullmatch(r"sha256:[0-9a-f]{64}", digest):
        raise Refused(f"artifact {artifact.get('id')} has no SHA-256 digest")
    size = artifact.get("size_in_bytes")
    if not isinstance(size, int) or not 0 < size <= MAX_ARTIFACT_BYTES:
        raise Refused(f"artifact {artifact.get('id')} size {size} is outside the {MAX_ARTIFACT_BYTES}-byte bound")
    return {"run_id": run["id"], "run_attempt": run.get("run_attempt"), "event": "merge_group",
            "conclusion": "success", "jobs": {n: conclusions[n] for n in REQUIRED_JOBS},
            "artifact_id": artifact["id"], "artifact_digest": digest, "artifact_size": size}


def collect_source(source):
    """Exact source identity. The source must already be part of main."""
    if gh_json(f"repos/{REPO}/compare/{source}...main").get("status") not in {"ahead", "identical"}:
        raise Refused(f"source {source} is not contained in main")
    commit = gh_json(f"repos/{REPO}/git/commits/{source}")
    if commit.get("sha") != source or not (commit.get("tree") or {}).get("sha"):
        raise Refused(f"source commit {source} has no readable tree")
    tree = gh_json(f"repos/{REPO}/git/trees/{commit['tree']['sha']}?recursive=1")
    if tree.get("truncated") or not isinstance(tree.get("tree"), list):
        raise Refused(f"source tree of {source} is truncated or unreadable")
    blobs = {e["path"]: e["sha"] for e in tree["tree"] if e.get("type") == "blob"}
    return {"sha": source, "tree_sha": commit["tree"]["sha"]}, blobs


def read_tar_members(archive_bytes, prefix):
    """Read untrusted archive bytes without extracting or executing them."""
    members = {}
    total = 0
    try:
        with tarfile.open(fileobj=io.BytesIO(archive_bytes), mode="r:gz") as tar:
            for member in tar:
                name = member.name
                if name != prefix and not name.startswith(prefix + "/"):
                    raise Refused(f"archive member {name} is outside {prefix}")
                relative = name[len(prefix) + 1:]
                if ".." in relative.split("/"):
                    raise Refused(f"archive member {name} escapes the release")
                if member.isdir():
                    continue
                if not member.isreg():
                    raise Refused(f"archive member {name} is not a regular file")
                total += member.size
                if total > MAX_ARTIFACT_BYTES:
                    raise Refused("archive contents exceed the size bound")
                members[relative] = tar.extractfile(member).read()
    except (tarfile.TarError, EOFError, OSError) as error:
        raise Refused(f"candidate archive unreadable: {error}")
    return members


def verify_artifact(artifact_bytes, ci, source, blobs, allow_no_ui):
    """Bind GitHub artifact, release archive, executable, packaged docs and UI to the source tree."""
    if f"sha256:{sha256_hex(artifact_bytes)}" != ci["artifact_digest"]:
        raise Refused("downloaded artifact does not match the digest GitHub reports")
    try:
        with zipfile.ZipFile(io.BytesIO(artifact_bytes)) as artifact:
            names = set(artifact.namelist())
            archives = [n for n in names if n.endswith(".tar.gz")]
            if len(archives) != 1 or names != {archives[0], "SHA256SUMS"}:
                raise Refused(f"artifact must contain one archive and SHA256SUMS, found {sorted(names)}")
            if sum(i.file_size for i in artifact.infolist()) > MAX_ARTIFACT_BYTES:
                raise Refused("artifact contents exceed the size bound")
            archive_name = archives[0]
            archive_bytes = artifact.read(archive_name)
            sums = artifact.read("SHA256SUMS").decode()
    except (zipfile.BadZipFile, UnicodeDecodeError) as error:
        raise Refused(f"artifact unreadable: {error}")
    match = re.fullmatch(rf"assetq-([0-9A-Za-z.-]+)-{source[:12]}-{re.escape(TARGET)}\.tar\.gz", archive_name)
    if not match:
        raise Refused(f"archive {archive_name} does not name source {source[:12]} and target {TARGET}")
    archive_sha = sha256_hex(archive_bytes)
    if sums.split() != [archive_sha, archive_name]:
        raise Refused("archive digest differs from artifact SHA256SUMS")
    members = read_tar_members(archive_bytes, archive_name[:-len(".tar.gz")])
    for required in ("bin/assetq", "candidate.json", "SHA256SUMS"):
        if required not in members:
            raise Refused(f"archive lacks {required}")
    try:
        manifest = json.loads(members["candidate.json"])
        inner = members["SHA256SUMS"].decode().split()
    except (ValueError, UnicodeDecodeError) as error:
        raise Refused(f"candidate metadata unreadable: {error}")
    binary = members["bin/assetq"]
    binary_sha = sha256_hex(binary)
    if inner != [binary_sha, "bin/assetq"] or manifest.get("binary_sha256") != binary_sha:
        raise Refused("executable digest differs from candidate metadata")
    if manifest.get("source_sha") != source:
        raise Refused(f"candidate source {manifest.get('source_sha')} differs from {source}")
    if manifest.get("target") != TARGET or manifest.get("version") != match.group(1):
        raise Refused("candidate target or version differs from its archive name")
    if not isinstance(manifest.get("schema"), int):
        raise Refused("candidate does not record its schema")
    libc = manifest.get("libc")
    if not (isinstance(libc, list) and len(libc) == 2 and libc[0] == "glibc" and re.fullmatch(r"\d+(\.\d+)+", str(libc[1]))):
        raise Refused("candidate does not record a glibc build version")
    for relative, data in members.items():
        if relative not in ("bin/assetq", "candidate.json", "SHA256SUMS") and blobs.get(relative) != git_blob_sha(data):
            raise Refused(f"packaged {relative} differs from the source tree")
    ui = verify_ui(manifest.get("embedded_ui"), blobs, binary, allow_no_ui)
    # The existing host names immutable releases by executable digest.
    return {"release_id": binary_sha, "archive_name": archive_name, "archive_sha256": archive_sha,
            "binary_sha256": binary_sha, "candidate_json_sha256": sha256_hex(members["candidate.json"]),
            "version": match.group(1), "schema": manifest["schema"], "target": TARGET, "libc": libc[1], "ui": ui}, archive_bytes


def verify_ui(embedded, blobs, binary, allow_no_ui):
    source_ui = {p[len("web/"):]: s for p, s in blobs.items() if re.fullmatch(r"web/[^/]+", p)}
    embedded = embedded or {}
    if not source_ui and not embedded:
        if not allow_no_ui:
            raise Refused("source and candidate carry no UI; pass --allow-no-ui to record that explicitly")
        return {"state": "none-in-source", "files": {}}
    if set(source_ui) != set(embedded):
        raise Refused(f"embedded UI files {sorted(embedded)} differ from source web files {sorted(source_ui)}")
    for name, blob_sha in sorted(source_ui.items()):
        reply = gh_json(f"repos/{REPO}/git/blobs/{blob_sha}")
        try:
            data = base64.b64decode(reply["content"])
        except (KeyError, ValueError, TypeError):
            raise Refused(f"source UI blob {name} unreadable")
        if git_blob_sha(data) != blob_sha or sha256_hex(data) != embedded[name]:
            raise Refused(f"embedded UI digest of {name} differs from the source blob")
        if data not in binary:
            raise Refused(f"executable does not embed the source bytes of {name}")
    return {"state": "bound", "files": dict(sorted(embedded.items()))}


# ---------------------------------------------------------------- host


class Host:
    """One SSH target reached as root. Host keys come from the operator's existing known_hosts; no key is ever created."""

    def __init__(self, target):
        self.target = target
        self.root = target["install_root"]

    def run(self, script, stdin_path=None, timeout=120):
        remote = "sh -c " + shlex.quote(script)
        argv = SSH + ["-o", "BatchMode=yes", "-o", "StrictHostKeyChecking=yes", "-o", "ConnectTimeout=10"]
        if "ssh_user" in self.target:
            argv += ["-l", self.target["ssh_user"]]
        argv += [self.target["ssh_alias"], remote]
        stdin = open(stdin_path, "rb") if stdin_path else subprocess.DEVNULL
        try:
            result = subprocess.run(argv, stdin=stdin, capture_output=True, timeout=timeout)
        except (OSError, subprocess.TimeoutExpired) as error:
            raise Transport(f"SSH did not complete: {type(error).__name__}")
        finally:
            if stdin_path:
                stdin.close()
        if result.returncode == 255:
            raise Transport(f"SSH verification or transport failed: {result.stderr.decode(errors='replace').strip()[:300]}")
        return result.returncode, result.stdout.decode(errors="replace"), result.stderr.decode(errors="replace").strip()[:400]

    def read(self, script, timeout=120):
        code, out, err = self.run(script, timeout=timeout)
        if code:
            raise Refused(f"host check failed ({code}): {err}")
        return out

    def user_command(self, command):
        """The service's own user manager, never the system manager or another account."""
        target = self.target
        return f"runuser -u {shlex.quote(target['service_account'])} -- env XDG_RUNTIME_DIR=/run/user/{target['service_uid']} {command}"

    def service(self, binary, arguments, seconds, env_file=None):
        """Run a release executable under the service's user manager; the operator never reads the environment file."""
        options = ["--user", "--wait", "--pipe", "--collect", "--quiet", f"--property=RuntimeMaxSec={seconds}"]
        if env_file:
            options.append(f"--property=EnvironmentFile={env_file}")
        command = " ".join(["systemd-run"] + [shlex.quote(o) for o in options] + [shlex.quote(binary)] + [shlex.quote(a) for a in arguments])
        return self.run(f"set -eu\nexec {self.user_command(command)}\n", timeout=seconds + 30)


def fields(text, keys):
    """One known key=value per line. A value spanning lines refuses; it never truncates or forges a key (R4)."""
    values = {}
    for line in text.splitlines():
        key, separator, value = line.partition("=")
        # systemd repeats EnvironmentFiles per file; inspect_host refuses more than one.
        if not separator or key not in keys or (key in values and key != "EnvironmentFiles"):
            raise Refused("host output is not one key=value per line")
        values.setdefault(key, value)
    return values


def release_of(link, root):
    link = link.strip()
    if link.startswith(root + "/"):
        link = link[len(root) + 1:]
    match = re.fullmatch(r"releases/([A-Za-z0-9._-]+)/?", link)
    return match.group(1) if match and match.group(1) not in {".", ".."} else None


def inspect_host(host):
    """Read-only identity and lifecycle observation. A missing value refuses instead of guessing (R4)."""
    target = host.target
    systemctl = host.user_command("systemctl --user")
    unit = shlex.quote(target["unit"])
    root = shlex.quote(host.root)
    account = shlex.quote(target["service_account"])
    props = " ".join(f"-p {p}" for p in UNIT_PROPS)
    tools = " ".join(HOST_TOOLS)
    base = host.read(f"""set -eu
for tool in {tools}; do command -v "$tool" >/dev/null || {{ echo "host tool missing: $tool" >&2; exit 40; }}; done
[ "$(id -u)" = 0 ] || {{ echo "operator SSH must reach the host as root" >&2; exit 47; }}
[ "$(id -u {account})" = {target['service_uid']} ] || {{ echo "service account uid differs from the target" >&2; exit 48; }}
cd {root}
v=$(hostname); printf 'hostname=%s\\n' "$v"
v=$(sha256sum {ETC}/machine-id | cut -d' ' -f1); printf 'machine_id_sha256=%s\\n' "$v"
v=$(ssh-keygen -E sha256 -lf {ETC}/ssh/ssh_host_ed25519_key.pub | cut -d' ' -f2); printf 'ssh_host_key_sha256=%s\\n' "$v"
v=$(getconf GNU_LIBC_VERSION | cut -d' ' -f2); printf 'libc=%s\\n' "$v"
v=$(readlink current); printf 'current=%s\\n' "$v"
v=$(readlink previous || true); printf 'previous=%s\\n' "$v"
v=$(sha256sum current/{shlex.quote(target["executable"])} | cut -d' ' -f1); printf 'current_binary_sha256=%s\\n' "$v"
u=$({systemctl} cat {unit}); v=$(printf '%s\\n' "$u" | sha256sum | cut -d' ' -f1); printf 'unit_sha256=%s\\n' "$v"
v=$(df -B1 --output=avail . | sed -n 2p | tr -d ' '); printf 'install_free=%s\\n' "$v"
v=$(df -B1 --output=avail {shlex.quote(target["data_root"])} | sed -n 2p | tr -d ' '); printf 'data_free=%s\\n' "$v"
v=$(stat -c '%U %G %a %d %i' {shlex.quote(target["data_root"])}); printf 'data_root_stat=%s\\n' "$v"
{systemctl} show {unit} {props}
""")
    seen = fields(base, INSPECT_KEYS + UNIT_PROPS)
    env_files = [line.split("=", 1)[1].split(" (")[0] for line in base.splitlines() if line.startswith("EnvironmentFiles=")]
    current = release_of(seen.get("current", ""), host.root)
    previous = release_of(seen.get("previous", ""), host.root) if seen.get("previous") else ""
    if not current or previous is None:
        raise Refused("host current/previous are not releases under the install root")
    match = re.search(r"argv\[\]=([^;]*);", seen.get("ExecStart", ""))
    exec_start = match.group(1).strip() if match else ""
    # A resource wrapper may precede the executable; the whole command stays bound below.
    wanted = r"(\S+( \S+)* )?" + re.escape(f"{host.root}/current/{target['executable']}") + rf" serve --listen 127\.0\.0\.1:{target['port']}( --http-threads \d+)?"
    if not re.fullmatch(wanted, exec_start):
        raise Refused(f"unit does not run current/{target['executable']} on loopback port {target['port']}: {exec_start or 'unknown'}")
    if len(env_files) != 1:
        raise Refused("unit must name exactly one environment file")
    stats = {}
    for path in dict.fromkeys(env_files + list(target["status_env_files"])):
        stats[path] = host.read(f"set -eu\nstat -c '%U %G %a %s %Y %d %i' {shlex.quote(path)}\n").strip()
        owner, _, mode = (stats[path].split() + ["", "", ""])[:3]
        if not re.fullmatch(r"[0-7]?[0-7][0-4]0", mode) or owner not in {"root", target["service_account"]}:
            raise Refused(f"protected file {path} has mode {mode} and owner {owner}")
    binding = {"current": current, "previous": previous, "current_binary_sha256": seen.get("current_binary_sha256"),
               "unit_sha256": seen.get("unit_sha256"), "exec_start": exec_start,
               "limits": {p: seen.get(p, "") for p in LIMIT_PROPS},
               "env_file": env_files[0], "protected_stats": stats, "data_root_stat": seen.get("data_root_stat")}
    identity = {k: seen.get(k) for k in ("hostname", "machine_id_sha256", "ssh_host_key_sha256")}
    if identity != target["expected"]:
        raise Refused(f"host identity {identity} differs from the expected target")
    if not all(isinstance(v, str) and v for v in (binding["current_binary_sha256"], binding["unit_sha256"], binding["data_root_stat"])):
        raise Refused("host lifecycle observation is incomplete")
    if not seen.get("install_free", "").isdigit() or not seen.get("data_free", "").isdigit():
        raise Refused("host free space is unknown")
    if int(seen["install_free"]) < target["min_install_free_bytes"]:
        raise Refused(f"install filesystem has {seen['install_free']} free bytes, below the configured floor")
    if int(seen["data_free"]) < target["min_data_free_bytes"]:
        raise Refused(f"data filesystem has {seen['data_free']} free bytes, below the configured floor")
    return {"identity": identity, "libc": seen.get("libc", ""), "binding": binding,
            "pid": seen.get("MainPID", ""), "active": seen.get("ActiveState", ""),
            "free": {"install_bytes": int(seen["install_free"]), "data_bytes": int(seen["data_free"])}}


def version_tuple(text):
    return tuple(int(part) for part in text.split("."))


def check_candidate_fits(host_state, candidate):
    if not re.fullmatch(r"\d+(\.\d+)+", host_state["libc"]) or version_tuple(host_state["libc"]) < version_tuple(candidate["libc"]):
        raise Refused(f"host glibc {host_state['libc'] or 'unknown'} is below candidate glibc {candidate['libc']}")
    if candidate["release_id"] == host_state["binding"]["current"]:
        raise Refused(f"release {candidate['release_id']} is already current")


def check_schema(observed, candidate):
    """The current executable just read this schema, so rollback to it stays compatible."""
    if observed["schema"] != candidate["schema"]:
        raise Refused(f"candidate schema {candidate['schema']} differs from current schema {observed['schema']}; this tool only applies same-schema candidates")


def integrity(host, binding, binary):
    code, out, err = host.service(binary, ["restore-check"], SERVICE_SECONDS, binding["env_file"])
    if code:
        raise Refused(f"current-store integrity check failed: {err}")
    try:
        report = json.loads(out)
    except ValueError:
        raise Refused("current-store integrity report unreadable")
    if report.get("verified") is not True or not all(isinstance(report.get(k), int) for k in MONOTONIC + ("schema",)):
        raise Refused("current-store integrity is not verified")
    return {k: report[k] for k in MONOTONIC + ("schema",)}


def active_work(host, binding, binary):
    """Project-scoped status through protected host files. No named file means unknown, never idle."""
    files = host.target["status_env_files"]
    if not files:
        return {"state": "unknown", "reason": "target names no protected status configuration", "projects": []}
    projects = []
    state = "idle"
    for path in files:
        code, out, err = host.service(binary, ["ops"], 60, path)
        try:
            jobs = json.loads(out)["jobs"] if code == 0 else None
            active = sum(int(v) for k, v in jobs["by_phase"].items() if k not in TERMINAL_PHASES)
            uncertain, leased = int(jobs["uncertain_submissions"]), int(jobs["leased"])
        except (ValueError, KeyError, TypeError, AttributeError):
            return {"state": "unknown", "reason": f"status unavailable: {err or 'unreadable report'}", "projects": projects}
        projects.append({"active": active, "uncertain": uncertain, "leased": leased})
        if active or uncertain or leased:
            state = "active"
    return {"state": state, "projects": projects}


def observe_service(host, expected_sha, version, before_pid):
    """Return (pid, None) for a healthy process running the exact executable, else (None, reason)."""
    target = host.target
    unit = shlex.quote(target["unit"])
    systemctl = host.user_command("systemctl --user")
    code, out, err = host.run(f"""set -eu
s=$({systemctl} show {unit} -p ActiveState -p MainPID); printf '%s\\n' "$s"
pid=$(printf '%s\\n' "$s" | sed -n 's/^MainPID=//p')
v=$(sha256sum {PROC}/"$pid"/exe | cut -d' ' -f1); printf 'exe=%s\\n' "$v"
v=$(curl -fsS --max-time 5 http://127.0.0.1:{target['port']}/v1/health); printf 'health=%s\\n' "$v"
""")
    if code:
        return None, f"health read failed: {err}"
    # The health body is the last field and spans lines: the service pretty-prints its JSON.
    report, _, body = out.partition("\nhealth=")
    try:
        seen = fields(report, ("exe",) + UNIT_PROPS)
    except Refused as error:
        return None, str(error)
    try:
        health = json.loads(body)
    except ValueError:
        return None, "health response unreadable"
    pid = seen.get("MainPID", "")
    if seen.get("ActiveState") != "active" or not pid or pid == "0":
        return None, f"unit is {seen.get('ActiveState')}"
    if before_pid is not None and pid == before_pid:
        return None, "unit still runs the process from before the restart"
    if seen.get("exe") != expected_sha:
        return None, "running executable differs from the expected digest"
    if health.get("service") != "up" or health.get("database") != "ok" or (version and health.get("version") != version):
        return None, f"health reported {health}"
    return pid, None


def wait_healthy(host, expected_sha, version, before_pid):
    """Two consecutive observations of one process, bounded by the target's health timeout."""
    attempts = max(2, host.target["health_timeout_seconds"] // HEALTH_INTERVAL_SECONDS)
    stable = None
    reason = "no observation"
    lost = False
    for _ in range(attempts):
        try:
            pid, reason = observe_service(host, expected_sha, version, before_pid)
            lost = False
        except Transport as error:
            pid, reason, lost = None, str(error), True
        if pid and pid == stable:
            return pid, None
        stable = pid
        time.sleep(HEALTH_INTERVAL_SECONDS)
    if lost:
        raise Transport(reason)
    return None, reason


def compare_binding(expected, actual, ignore=()):
    return [k for k in sorted(set(expected) | set(actual)) if k not in ignore and expected.get(k) != actual.get(k)]


# ---------------------------------------------------------------- plan


def seal(plan):
    plan = {k: v for k, v in plan.items() if k != "plan_sha256"}
    plan["plan_sha256"] = sha256_hex(canonical(plan))
    return plan


def load_plan(path, confirm):
    try:
        plan = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise Refused(f"plan unreadable: {error}")
    if not isinstance(plan, dict) or plan.get("format") != FORMAT:
        raise Refused(f"plan format is not {FORMAT}")
    if seal(plan)["plan_sha256"] != plan.get("plan_sha256"):
        raise Refused("plan content differs from its digest")
    if plan["plan_sha256"] != confirm:
        raise Refused(f"--confirm-plan differs from reviewed plan digest {plan['plan_sha256']}")
    return plan


def check_fresh(plan):
    moment = now()
    created = datetime.datetime.strptime(plan["created_at"], "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=datetime.timezone.utc)
    expires = datetime.datetime.strptime(plan["expires_at"], "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=datetime.timezone.utc)
    if moment >= expires or moment < created - datetime.timedelta(minutes=5):
        raise Refused(f"plan is stale: valid {plan['created_at']} to {plan['expires_at']}; create and review a new plan")


def short(release):
    """A phrase names a release briefly; the plan digest binds the full identity."""
    return release[:12]


def action_phrases(candidate, hostname, unit, prior):
    release = short(candidate["release_id"])
    return [
        {"id": "prepare", "effect": "Upload and verify an immutable release beside the current one; no service change.",
         "phrase": f"prepare {release} on {hostname}"},
        {"id": "migrate", "effect": "Run the candidate's additive same-schema migrate with the service identity; the old process keeps running.",
         "phrase": f"migrate {release} on {hostname}"},
        {"id": "activate", "effect": f"Switch current, restart {unit} (service outage until healthy) and roll back to {prior} automatically on failed verification.",
         "phrase": f"activate {release} on {hostname} restarting {unit} with rollback to {short(prior)}"},
    ]


def make_plan(args):
    target = load_target(args.target)
    out = Path(args.out)
    if out.exists():
        raise Refused(f"output directory {out} already exists")
    ci = collect_ci(args.source)
    source, blobs = collect_source(args.source)
    artifact_bytes = gh_bytes(f"repos/{REPO}/actions/artifacts/{ci['artifact_id']}/zip")
    candidate, archive_bytes = verify_artifact(artifact_bytes, ci, args.source, blobs, args.allow_no_ui)
    host = Host(target)
    state = inspect_host(host)
    check_candidate_fits(state, candidate)
    binding = state["binding"]
    binary = f"{host.root}/current/{target['executable']}"
    pid, reason = observe_service(host, binding["current_binary_sha256"], None, None)
    if not pid:
        raise Refused(f"existing service is not verifiably healthy, so rollback has no verified target: {reason}")
    observed = integrity(host, binding, binary)
    check_schema(observed, candidate)
    work = active_work(host, binding, binary)
    if work["state"] != "idle" and not args.accept_active_work:
        raise Refused(f"active work is {work['state']} ({work}); pass --accept-active-work only after reviewing it")
    created = now()
    plan = seal({
        "format": FORMAT, "repo": REPO, "created_at": stamp(created),
        "expires_at": stamp(created + datetime.timedelta(seconds=PLAN_TTL_SECONDS)),
        "target": target, "source": source, "ci": ci, "candidate": candidate,
        "host": {"identity": state["identity"], "libc": state["libc"], "binding": binding, "free": state["free"],
                 "integrity": observed, "active_work": work},
        "rollback": {"release_id": binding["current"], "binary_sha256": binding["current_binary_sha256"], "previous_release_id": binding["previous"]},
        "accepted_active_work": bool(args.accept_active_work),
        "actions": action_phrases(candidate, state["identity"]["hostname"], target["unit"], binding["current"]),
    })
    out.mkdir(parents=True, mode=0o700)
    (out / candidate["archive_name"]).write_bytes(archive_bytes)
    (out / "plan.json").write_text(json.dumps(plan, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({"result": "planned; no host effect", "plan": str(out / "plan.json"), "plan_sha256": plan["plan_sha256"],
                      "release": candidate["release_id"], "rollback": binding["current"], "active_work": work,
                      "expires_at": plan["expires_at"], "actions": plan["actions"]}, indent=2))
    return 0


# ---------------------------------------------------------------- apply and rollback


class Journal:
    """Append-only local receipt. Intent is durable before each effect; one plan applies once."""

    def __init__(self, path):
        try:
            self.stream = open(path, "x", encoding="utf-8")
        except FileExistsError:
            raise Refused(f"{path} exists: this plan was already used. Inspect it and create a new plan")

    def record(self, step, status, **detail):
        self.stream.write(json.dumps(dict(at=stamp(now()), step=step, status=status, **detail), sort_keys=True) + "\n")
        self.stream.flush()
        os.fsync(self.stream.fileno())


def require_operator():
    for name in ("CI", "GITHUB_ACTIONS", "TASKQ_TASK"):
        if os.environ.get(name):
            raise Refused(f"LIVE actions refuse automation ({name} is set); an operator runs this from their own terminal")


def ask(phrase, effect):
    """Per-action LIVE confirmation; a non-interactive caller cannot supply it."""
    if not sys.stdin.isatty():
        raise Refused("LIVE confirmation needs an interactive terminal")
    print(f"\nLIVE action: {effect}\nType exactly to proceed:\n  {phrase}", file=sys.stderr)
    try:
        answer = input("> ").strip()
    except (EOFError, KeyboardInterrupt):
        answer = None
    if answer != phrase:
        raise Refused(f"confirmation did not match: {phrase}")


def verify_plan_now(plan, plan_path):
    """Fresh evidence at apply time; any drift from the reviewed plan refuses."""
    check_fresh(plan)
    source = plan["source"]["sha"]
    archive = Path(plan_path).parent / plan["candidate"]["archive_name"]
    if not archive.is_file() or sha256_file(archive) != plan["candidate"]["archive_sha256"]:
        raise Refused(f"local candidate archive {archive.name} is missing or differs from the plan")
    fresh_ci = collect_ci(source)
    if fresh_ci != plan["ci"]:
        raise Refused(f"CI evidence changed since the plan: {compare_binding(plan['ci'], fresh_ci)}")
    fresh_source, _ = collect_source(source)
    if fresh_source != plan["source"]:
        raise Refused("source evidence changed since the plan")
    host = Host(plan["target"])
    state = inspect_host(host)
    check_candidate_fits(state, plan["candidate"])
    drift = compare_binding(plan["host"]["binding"], state["binding"])
    if drift:
        raise Refused(f"host changed since the plan: {drift}")
    current_binary = f"{host.root}/current/{host.target['executable']}"
    check_schema(integrity(host, state["binding"], current_binary), plan["candidate"])
    work = active_work(host, state["binding"], current_binary)
    if work["state"] != "idle" and not plan["accepted_active_work"]:
        raise Refused(f"active work is now {work['state']}; the plan did not accept it")
    return host, state, archive, work


def acquire_lock(host, digest, journal):
    try:
        code, _, err = host.run(f"set -eu\ncd {shlex.quote(host.root)}\nmkdir deploy.lock\nprintf '%s\\n' {digest} > deploy.lock/plan\n")
    except Transport as error:
        journal.record("lock", "uncertain", reason=str(error))
        raise Uncertain(f"deploy.lock state unknown: {error}")
    if code:
        journal.record("lock", "refused", reason=err)
        raise Refused(f"another deployment or an unresolved one holds deploy.lock: {err}")


def release_lock(host):
    code, _, err = host.run(f"set -eu\ncd {shlex.quote(host.root)}\nrm -f deploy.lock/plan\nrmdir deploy.lock\n")
    if code:
        raise Uncertain(f"deploy.lock could not be released: {err}")


def prepare_release(host, plan, archive, binding, journal):
    """CoreQ R15 prepare: verified immutable bytes without touching current, previous or units."""
    candidate = plan["candidate"]
    release = shlex.quote(candidate["release_id"])
    root = shlex.quote(host.root)
    executable = shlex.quote(host.target["executable"])
    code, out, err = host.run(f"""set -eu
umask 022
cd {root}/releases
if [ -e {release} ]; then
  b=$(sha256sum {release}/{executable} | cut -d' ' -f1); c=$(sha256sum {release}/candidate.json | cut -d' ' -f1)
  if [ "$b" = {candidate['binary_sha256']} ] && [ "$c" = {candidate['candidate_json_sha256']} ]; then echo reuse; exit 0; fi
  echo "release exists with different bytes; preserved" >&2; exit 41
fi
mkdir {release}.tmp
echo new
""")
    if code:
        raise Refused(f"release preparation refused: {err}")
    if out.strip() == "reuse":
        journal.record("prepare", "reused-identical")
        return
    temporary = f"{root}/releases/{release}.tmp"
    code, _, err = host.run(f"set -eu\ncat > {temporary}/archive.tar.gz\n", stdin_path=str(archive), timeout=900)
    if code == 0:
        code, _, err = host.run(f"""set -eu
umask 022
cd {temporary}
a=$(sha256sum archive.tar.gz | cut -d' ' -f1); [ "$a" = {candidate['archive_sha256']} ] || {{ echo "uploaded archive digest differs" >&2; exit 42; }}
tar --no-same-owner --strip-components=1 -xzf archive.tar.gz
rm archive.tar.gz
sha256sum -c SHA256SUMS >/dev/null
b=$(sha256sum bin/assetq | cut -d' ' -f1); [ "$b" = {candidate['binary_sha256']} ] || {{ echo "extracted executable digest differs" >&2; exit 43; }}
c=$(sha256sum candidate.json | cut -d' ' -f1); [ "$c" = {candidate['candidate_json_sha256']} ] || {{ echo "extracted metadata digest differs" >&2; exit 44; }}
[ -e {executable} ] || cp bin/assetq {executable}
chmod -R a+rX,go-w .
chown -R --reference={root}/releases/{shlex.quote(binding['current'])} .
""", timeout=300)
    if code == 0:
        code, _, err = host.service(f"{host.root}/releases/{candidate['release_id']}.tmp/{host.target['executable']}", ["selftest"], SELFTEST_SECONDS)
    if code == 0:
        code, _, err = host.run(f"set -eu\ncd {root}/releases\nmv -T {release}.tmp {release}\n")
    if code:
        host.run(f"rm -rf {temporary}")
        raise Refused(f"release preparation failed; temporary files removed: {err}")
    journal.record("prepare", "done")


def switch_release(host, to_release, expected_current, new_previous):
    """Atomic rename of one-version pointers (CoreQ R12 step 6, R13); releases stay in place."""
    root = shlex.quote(host.root)
    executable = shlex.quote(host.target["executable"])
    if new_previous:
        previous = f"ln -sfn {root}/releases/{shlex.quote(new_previous)} previous.tmp-$$\nmv -T previous.tmp-$$ previous\n"
    else:
        previous = "rm -f previous\n"
    return host.run(f"""set -eu
cd {root}
[ -x releases/{shlex.quote(to_release)}/{executable} ] || {{ echo "release {to_release} is absent" >&2; exit 45; }}
c=$(readlink current); [ "${{c##*/}}" = {shlex.quote(expected_current)} ] || {{ echo "current is $c, expected {expected_current}" >&2; exit 46; }}
{previous}ln -sfn {root}/releases/{shlex.quote(to_release)} current.tmp-$$
mv -T current.tmp-$$ current
""")


def restart(host):
    return host.run(host.user_command(f"systemctl --user restart {shlex.quote(host.target['unit'])}"), timeout=240)


def verify_activation(host, plan, baseline, before_pid):
    """Return None when the candidate runs and data, jobs and guards are preserved, else a reason."""
    candidate = plan["candidate"]
    pid, reason = wait_healthy(host, candidate["binary_sha256"], candidate["version"], before_pid)
    if not pid:
        return reason
    binding = plan["host"]["binding"]
    try:
        state = inspect_host(host)
    except Refused as error:
        return str(error)
    drift = compare_binding(binding, state["binding"], ignore=("current", "previous", "current_binary_sha256"))
    if drift:
        return f"service configuration changed: {drift}"
    if state["binding"]["current"] != candidate["release_id"] or state["binding"]["current_binary_sha256"] != candidate["binary_sha256"]:
        return "current does not name the candidate"
    try:
        after = integrity(host, binding, f"{host.root}/current/{host.target['executable']}")
    except Refused as error:
        return str(error)
    decreased = [k for k in MONOTONIC if after[k] < baseline[k]]
    return f"retained state decreased: {decreased}" if decreased else None


def restore_previous(host, plan, journal, reason, before_pid):
    """Roll back to the plan's preserved release. An unverified result is uncertain, never success."""
    rollback = plan["rollback"]
    candidate = plan["candidate"]
    journal.record("rollback", "intent", reason=reason)
    try:
        code, _, err = switch_release(host, rollback["release_id"], candidate["release_id"], rollback["previous_release_id"])
        if code:
            raise Uncertain(f"rollback pointer switch failed after: {reason}. {err}")
        code, _, err = restart(host)
        pid, failure = wait_healthy(host, rollback["binary_sha256"], None, before_pid)
    except Transport as error:
        journal.record("rollback", "uncertain", reason=str(error))
        raise Uncertain(f"rollback outcome unknown after: {reason}. {error}")
    if not pid:
        journal.record("rollback", "uncertain", reason=failure)
        raise Uncertain(f"rollback did not verify after: {reason}. {failure}")
    try:
        after = integrity(host, plan["host"]["binding"], f"{host.root}/current/{host.target['executable']}")
    except (Refused, Transport) as error:
        journal.record("rollback", "uncertain", reason=str(error))
        raise Uncertain(f"rollback integrity unknown after: {reason}. {error}")
    decreased = [k for k in MONOTONIC if after[k] < plan["host"]["integrity"][k]]
    if decreased:
        journal.record("rollback", "uncertain", decreased=decreased)
        raise Uncertain(f"rollback verified the process, but retained state decreased: {decreased}")
    journal.record("rollback", "done", integrity=after)
    return pid


def apply_plan(args):
    plan = load_plan(args.plan, args.confirm_plan)
    host, state, archive, work = verify_plan_now(plan, args.plan)
    if args.dry_run:
        print(json.dumps({"result": "verified; dry run made no host effect", "plan_sha256": plan["plan_sha256"],
                          "release": plan["candidate"]["release_id"], "rollback": plan["rollback"]["release_id"],
                          "active_work": work, "actions": plan["actions"]}, indent=2))
        return 0
    require_operator()
    actions = {a["id"]: a for a in plan["actions"]}
    ask(actions["prepare"]["phrase"], actions["prepare"]["effect"])
    journal = Journal(f"{args.plan}.journal")
    journal.record("plan", "accepted", plan_sha256=plan["plan_sha256"], active_work=work)
    binding = state["binding"]
    acquire_lock(host, plan["plan_sha256"], journal)
    activated = False
    try:
        prepare_release(host, plan, archive, binding, journal)
        ask(actions["migrate"]["phrase"], actions["migrate"]["effect"])
        baseline = integrity(host, binding, f"{host.root}/current/{host.target['executable']}")
        journal.record("migrate", "intent", baseline=baseline)
        code, _, err = host.service(f"{host.root}/releases/{plan['candidate']['release_id']}/{host.target['executable']}", ["migrate"], SERVICE_SECONDS,
                                    binding["env_file"])
        if code:
            raise Refused(f"migrate failed; current release and service unchanged: {err}")
        journal.record("migrate", "done")
        ask(actions["activate"]["phrase"], actions["activate"]["effect"])
        fresh = inspect_host(host)
        drift = compare_binding(binding, fresh["binding"])
        if drift:
            raise Refused(f"host changed before activation: {drift}")
        check_fresh(plan)
        before_pid = fresh["pid"]
        journal.record("activate", "intent", release=plan["candidate"]["release_id"], before_pid=before_pid)
        activated = True
        code, _, err = switch_release(host, plan["candidate"]["release_id"], binding["current"], binding["current"])
        if code:
            activated = False
            raise Refused(f"activation refused before any switch: {err}")
        code, _, err = restart(host)
        reason = f"restart failed: {err}" if code else verify_activation(host, plan, baseline, before_pid)
    except Refused as error:
        journal.record("stopped", "refused", reason=str(error))
        try:
            release_lock(host)
        except Transport as lost:
            raise Uncertain(f"{error}; lock release is unknown: {lost}")
        raise
    except (Transport, Uncertain) as error:
        journal.record("stopped", "uncertain" if activated else "transport", reason=str(error))
        if activated:
            raise Uncertain(f"activation outcome unknown; deploy.lock retained: {error}")
        raise Uncertain(f"host outcome unknown before activation; deploy.lock retained: {error}")
    if reason:
        journal.record("verify", "failed", reason=reason)
        pid = restore_previous(host, plan, journal, reason, None)
        try:
            release_lock(host)
        except Transport as error:
            raise Uncertain(f"rollback verified, but lock release is unknown: {error}")
        print(json.dumps({"result": "rolled back", "reason": reason, "running": plan["rollback"]["release_id"], "pid": pid}, indent=2))
        return EXIT_ROLLED_BACK
    try:
        code, _, err = host.run(f"set -eu\ntouch {shlex.quote(host.root)}/releases/{shlex.quote(plan['candidate']['release_id'])}/healthy\n")
        release_lock(host)
    except Transport as error:
        raise Uncertain(f"candidate verified, but healthy marker or lock release is unknown: {error}")
    journal.record("activate", "verified", healthy_marker="written" if code == 0 else f"failed: {err}")
    print(json.dumps({"result": "deployed and verified on this host", "release": plan["candidate"]["release_id"],
                      "binary_sha256": plan["candidate"]["binary_sha256"], "rollback": plan["rollback"]["release_id"],
                      "not_covered": "DNS, proxy, access and browser journeys remain separate actions"}, indent=2))
    return 0


def rollback_plan(args):
    """One command to the preserved previous release. Requires the applied candidate to be current."""
    plan = load_plan(args.plan, args.confirm_plan)
    host = Host(plan["target"])
    state = inspect_host(host)
    rollback = plan["rollback"]
    binding = state["binding"]
    if binding["current"] != plan["candidate"]["release_id"] or binding["current_binary_sha256"] != plan["candidate"]["binary_sha256"]:
        raise Refused(f"current is {binding['current']}, not this plan's candidate; rollback does not apply")
    drift = compare_binding(plan["host"]["binding"], binding, ignore=("current", "previous", "current_binary_sha256"))
    if drift:
        raise Refused(f"service configuration changed since the plan: {drift}")
    code, out, err = host.run(f"set -eu\nsha256sum {shlex.quote(host.root)}/releases/{shlex.quote(rollback['release_id'])}/{shlex.quote(host.target['executable'])} | cut -d' ' -f1\n")
    if code or out.strip() != rollback["binary_sha256"]:
        raise Refused(f"preserved release {rollback['release_id']} is missing or differs: {err}")
    work = active_work(host, binding, f"{host.root}/current/{host.target['executable']}")
    phrase = f"rollback {host.target['unit']} on {state['identity']['hostname']} to {short(rollback['release_id'])}"
    if args.dry_run:
        print(json.dumps({"result": "verified; dry run made no host effect", "phrase": phrase, "active_work": work}, indent=2))
        return 0
    require_operator()
    ask(phrase, f"Switch current to {rollback['release_id']} and restart {host.target['unit']}; active work is {work['state']}.")
    journal = Journal(f"{args.plan}.rollback-journal")
    journal.record("plan", "accepted", plan_sha256=plan["plan_sha256"], active_work=work)
    acquire_lock(host, plan["plan_sha256"], journal)
    pid = restore_previous(host, plan, journal, "operator rollback", state["pid"])
    try:
        release_lock(host)
    except Transport as error:
        raise Uncertain(f"rollback verified, but lock release is unknown: {error}")
    print(json.dumps({"result": "rolled back and verified", "running": rollback["release_id"], "pid": pid}, indent=2))
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest="command", required=True)
    plan = commands.add_parser("plan", help="verify evidence and write a reviewable plan; no host effect")
    plan.add_argument("--target", required=True, help="private target JSON; never committed")
    plan.add_argument("--source", required=True, help="full commit SHA already on main")
    plan.add_argument("--out", required=True, help="new private directory for plan.json and the candidate archive")
    plan.add_argument("--allow-no-ui", action="store_true", help="record explicitly that this source has no web UI")
    plan.add_argument("--accept-active-work", action="store_true", help="record reviewed active or unknown work in the plan")
    for name, helper in [("apply", "LIVE: prepare, migrate and activate exactly the reviewed plan"),
                         ("rollback", "LIVE: activate the plan's preserved release")]:
        command = commands.add_parser(name, help=helper)
        command.add_argument("--plan", required=True)
        command.add_argument("--confirm-plan", required=True, help="plan_sha256 printed by plan after review")
        command.add_argument("--dry-run", action="store_true", help="re-verify everything; no host effect")
    args = parser.parse_args(argv)
    handler = {"plan": make_plan, "apply": apply_plan, "rollback": rollback_plan}[args.command]
    try:
        return handler(args)
    except Refused as error:
        print(f"refused: {error}", file=sys.stderr)
        return EXIT_REFUSED
    except Transport as error:
        print(f"refused: {error}", file=sys.stderr)
        return EXIT_REFUSED
    except Uncertain as error:
        print(f"uncertain: {error}", file=sys.stderr)
        return EXIT_UNCERTAIN


if __name__ == "__main__":
    raise SystemExit(main())
