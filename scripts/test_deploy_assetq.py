"""Deployment tool checks (A2-A4) against invented GitHub evidence and a throwaway host tree.

Risks protected: a wrong source, hash or CI result reaching a host; a stale plan acting on a
changed host; a lost SSH connection reported as success; a failed activation left running;
data, guards or the preserved release changed by a deployment. Remote scripts run in real
POSIX sh with fake service tools. This proves tool logic only: never systemd, a real host,
real SSH trust or deployment."""
import base64
import contextlib
import datetime
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import zipfile

import deploy_assetq as deploy

SOURCE = "a1" * 20
OTHER = "b2" * 20
SECRET = "invented-secret-never-read"
OLD = "5a" * 32
UID = 995
DOC = b"# invented packaged contract\n"
UI = {"app.js": b"// invented ui script\n", "index.html": b"<!-- invented ui page -->\n"}

# Reachable only as the real host's lifecycle is: the service account, its runtime directory, --user.
USER_MANAGER = """[ "$SERVICE_ACCOUNT" = assetq ] && [ "$XDG_RUNTIME_DIR" = /run/user/995 ] && [ "$1" = --user ] || {
  echo "Failed to connect to bus: fixture requires the service user manager" >&2; exit 1; }
"""

TOOLS = {
    "hostname": 'echo invented-host\n',
    "ssh-keygen": 'echo "256 SHA256:invented-key comment (ED25519)"\n',
    "getconf": 'cat "$FIXTURE/state/libc"\n',
    "id": 'case "$*" in\n"-u") cat "$FIXTURE/state/operator_uid" ;;\n"-u assetq") cat "$FIXTURE/state/account_uid" ;;\n*) exit 9 ;;\nesac\n',
    "runuser": '[ "$1" = -u ] && [ "$3" = -- ] || exit 9\nexport SERVICE_ACCOUNT=$2\nshift 3\nexec "$@"\n',
    "chown": 'echo "chown $*" >> "$FIXTURE/log/tools"\n',
    "df": 'echo Avail\ncat "$FIXTURE/state/free"\n',
    "curl": 'exec "$FIXTURE/host/opt/current/$(cat "$FIXTURE/state/executable")" health\n',
    "stat": 'case "$2" in\n*%s*) cat "$FIXTURE/state/env_stat" ;;\n*) echo "assetq assetq 750 5 9" ;;\nesac\n',
    "systemd-run": (USER_MANAGER + 'echo "systemd-run $*" >> "$FIXTURE/log/tools"\n'
                    'while [ "${1#--}" != "$1" ]; do shift; done\nexec "$@"\n'),
    "systemctl": USER_MANAGER + r'''echo "systemctl $*" >> "$FIXTURE/log/tools"
shift
root="$FIXTURE/host/opt"
executable=$(cat "$FIXTURE/state/executable")
case "$1" in
cat) cat "$FIXTURE/state/unit" ;;
show)
  pid=$(cat "$FIXTURE/state/pid")
  printf 'ExecStart={ path=x ; argv[]=%s ; ignore_errors=no ; pid=%s ; }\n' "$(cat "$FIXTURE/state/exec")" "$pid"
  printf 'EnvironmentFiles=%s (ignore_errors=no)\n' "$FIXTURE/host/env/assetq.env"
  printf 'ActiveState=%s\nMainPID=%s\nLimitAS=2147483648\nLimitSTACK=2097152\nAllowedCPUs=0-1\nMemoryMax=infinity\nTasksMax=512\n' "$(cat "$FIXTURE/state/active")" "$pid"
  ;;
restart)
  pid=$(( $(cat "$FIXTURE/state/pid") + 1 ))
  release=$(readlink "$root/current"); release=${release##*/}
  if "$root/current/$executable" start; then
    mkdir -p "$FIXTURE/host/proc/$pid"
    ln -s "$root/releases/$release/$executable" "$FIXTURE/host/proc/$pid/exe"
    echo "$pid" > "$FIXTURE/state/pid"; echo active > "$FIXTURE/state/active"
  else
    echo 0 > "$FIXTURE/state/pid"; echo failed > "$FIXTURE/state/active"
  fi
  ;;
*) exit 9 ;;
esac
''',
    "fake-ssh": r'''for a; do remote=$a; done
printf '%s\n---\n' "$remote" >> "$FIXTURE/log/ssh"
if [ -f "$FIXTURE/state/ssh_fail" ]; then
  case "$remote" in *"$(cat "$FIXTURE/state/ssh_fail")"*) echo "Host key verification failed." >&2; exit 255 ;; esac
fi
exec sh -c "$remote"
''',
}


def executable(health="ok", selftest=0, start=0, integrity="integrity", migrate=0, marker="candidate", ui=True):
    """An invented service executable; `integrity` names the state file its restore-check reads."""
    embedded = "".join(f"# {data.decode()}" for data in UI.values()) if ui else ""
    health_case = ('cat "$FIXTURE/state/old_health"' if health == "state"
                   else 'echo \'{"service":"up","version":"0.1.0","database":"ok"}\'' if health == "ok"
                   else "exit 7")
    return (f'#!/bin/sh\n# {marker}\n{embedded}case "$1" in\n'
            f'selftest) exit {selftest} ;;\nstart) exit {start} ;;\nhealth) {health_case} ;;\n'
            f'migrate) echo "migrate {marker}" >> "$FIXTURE/log/tools"; exit {migrate} ;;\n'
            f'restore-check) cat "$FIXTURE/state/{integrity}" ;;\nops) cat "$FIXTURE/state/ops" ;;\nesac\n').encode()


def blob(data):
    return hashlib.sha1(b"blob %d\0" % len(data) + data).hexdigest()


def sha(data):
    return hashlib.sha256(data).hexdigest()


class Evidence:
    """Invented GitHub responses for one source. Each case edits one fact."""

    def __init__(self, binary=None, schema=1, libc="2.43", source=SOURCE, with_ui=True):
        self.source = source
        self.ui = dict(UI) if with_ui else {}
        self.binary = binary if binary is not None else executable(ui=with_ui)
        self.manifest = {"source_sha": source, "target": deploy.TARGET, "version": "0.1.0", "schema": schema,
                         "libc": ["glibc", libc], "binary_sha256": sha(self.binary),
                         "embedded_ui": {n: sha(d) for n, d in self.ui.items()}}
        if not with_ui:
            del self.manifest["embedded_ui"]
        self.doc_in_tree = DOC
        self.doc_in_archive = DOC
        self.inner_sums = None
        self.outer_sums = None
        self.extra_member = None
        self.run = {"id": 7, "event": "merge_group", "path": deploy.WORKFLOW, "head_sha": source, "status": "completed",
                    "conclusion": "success", "run_attempt": 1}
        self.runs = [self.run]
        self.jobs = [{"name": "tests", "conclusion": "success"}, {"name": "candidate", "conclusion": "success"}]
        self.artifact_edit = {}
        self.compare = "identical"
        self.zip_override = None

    def archive(self):
        name = f"assetq-0.1.0-{self.source[:12]}-{deploy.TARGET}"
        members = {"bin/assetq": self.binary, "candidate.json": json.dumps(self.manifest).encode(),
                   "SHA256SUMS": (self.inner_sums or f"{sha(self.binary)}  bin/assetq\n").encode(),
                   "docs/contract.md": self.doc_in_archive}
        stream = io.BytesIO()
        with gzip.GzipFile(fileobj=stream, mode="wb", mtime=0) as compressed, tarfile.open(fileobj=compressed, mode="w") as tar:
            for relative, data in members.items():
                info = tarfile.TarInfo(f"{name}/{relative}")
                info.size, info.mode = len(data), 0o755
                tar.addfile(info, io.BytesIO(data))
            if self.extra_member:
                tar.addfile(self.extra_member)
        return f"{name}.tar.gz", stream.getvalue()

    def zip(self):
        if self.zip_override is not None:
            return self.zip_override
        name, data = self.archive()
        stream = io.BytesIO()
        with zipfile.ZipFile(stream, "w") as artifact:
            artifact.writestr(zipfile.ZipInfo(name), data)
            artifact.writestr(zipfile.ZipInfo("SHA256SUMS"), self.outer_sums or f"{sha(data)}  {name}\n")
        return stream.getvalue()

    @property
    def release(self):
        return sha(self.binary)

    def __call__(self, path):
        repo = f"repos/{deploy.REPO}/"
        self.calls = getattr(self, "calls", []) + [path]
        if path.endswith("/zip"):
            return self.zip()
        reply = None
        if path.startswith(f"{repo}actions/runs?"):
            reply = {"workflow_runs": self.runs}
        elif "/jobs?" in path:
            reply = {"jobs": self.jobs}
        elif "/artifacts?" in path:
            data = self.zip()
            reply = {"artifacts": [dict({"id": 11, "name": deploy.ARTIFACT, "expired": False, "size_in_bytes": len(data),
                                         "digest": f"sha256:{sha(data)}",
                                         "workflow_run": {"id": self.run["id"], "head_sha": self.source}}, **self.artifact_edit)]}
        elif "/compare/" in path:
            reply = {"status": self.compare}
        elif "/git/commits/" in path:
            reply = {"sha": self.source, "tree": {"sha": "tree-1"}}
        elif "/git/trees/" in path:
            entries = [{"path": "docs/contract.md", "type": "blob", "sha": blob(self.doc_in_tree)}]
            entries += [{"path": f"web/{n}", "type": "blob", "sha": blob(d)} for n, d in self.ui.items()]
            reply = {"truncated": False, "tree": entries}
        elif "/git/blobs/" in path:
            wanted = path.rsplit("/", 1)[1]
            data = next(d for d in self.ui.values() if blob(d) == wanted)
            reply = {"content": base64.b64encode(data).decode()}
        if reply is None:
            raise AssertionError(f"unexpected GitHub path {path}")
        return json.dumps(reply).encode()


class Fixture(unittest.TestCase):
    def setUp(self):
        self.sh = shutil.which("sh")
        if not self.sh:
            self.fail("POSIX sh is required for deployment fixtures")
        self.base = Path(tempfile.mkdtemp(prefix="aq-deploy-"))
        self.addCleanup(shutil.rmtree, self.base, ignore_errors=True)
        self.posix = subprocess.run([self.sh, "-c", 'cd "$1" && pwd', "sh", str(self.base)], check=True,
                                    capture_output=True, text=True).stdout.strip()
        tools = self.base / "bin"
        tools.mkdir()
        for name, body in TOOLS.items():
            (tools / name).write_bytes(("#!/bin/sh\n" + body).encode())
            (tools / name).chmod(0o755)
        environment = {k: v for k, v in os.environ.items() if k not in ("CI", "GITHUB_ACTIONS", "TASKQ_TASK")}
        environment.update(FIXTURE=self.posix, MSYS="winsymlinks:nativestrict",
                           PATH=str(tools) + os.pathsep + os.environ["PATH"])
        # Fixture files are host (LF) bytes on every runner.
        for patcher in [patch.object(Path, "write_text", lambda path, data, encoding=None: path.write_bytes(data.encode())),
                        patch.dict(os.environ, environment, clear=True),
                        patch.object(deploy, "SSH", [self.sh, f"{self.posix}/bin/fake-ssh"]),
                        patch.object(deploy, "ETC", f"{self.posix}/host/etc"),
                        patch.object(deploy, "PROC", f"{self.posix}/host/proc"),
                        patch.object(deploy.time, "sleep", lambda _: None)]:
            patcher.start()
            self.addCleanup(patcher.stop)
        self.host = self.base / "host"
        self.state = self.base / "state"
        for directory in ["state", "log", "host/etc/ssh", "host/proc/100", "host/opt/releases", "host/data", "host/env"]:
            (self.base / directory).mkdir(parents=True)
        (self.host / "etc/machine-id").write_text("invented-machine\n")
        (self.host / "etc/ssh/ssh_host_ed25519_key.pub").write_text("invented\n")
        (self.host / "data/object").write_text("invented retained object\n")
        (self.host / "env/assetq.env").write_text(f"ASSETQ_DATABASE_URL={SECRET}\n")
        old = self.host / "opt/releases" / OLD
        (old / "bin").mkdir(parents=True)
        (old / "bin/assetq").write_bytes(executable(health="state", marker="old", ui=False))
        (old / "bin/assetq").chmod(0o755)
        counts = {"verified": True, "schema": 1, "jobs": 3, "succeeded": 2, "blobs": 4, "methods": 5, "attempts": 6, "physical_object_bytes": 70}
        for name, value in [("operator_uid", "0\n"), ("account_uid", f"{UID}\n"), ("executable", "bin/assetq\n"), ("libc", "glibc 2.43\n"), ("free", "  99999999\n"), ("env_stat", "root root 600 12 1700 5 8\n"),
                            ("unit", "[Service]\ninvented unit\n"), ("pid", "100\n"), ("active", "active\n"),
                            ("exec", f"{self.posix}/host/opt/current/bin/assetq serve --listen 127.0.0.1:8740\n"),
                            ("integrity", json.dumps(counts)),
                            ("integrity_low", json.dumps(dict(counts, blobs=1))),
                            ("old_health", '{"service":"up","version":"0.1.0","database":"ok"}\n'),
                            ("ops", json.dumps({"jobs": {"by_phase": {"succeeded": 2}, "uncertain_submissions": 0, "leased": 0}}))]:
            (self.state / name).write_text(value)
        self.sh_run(f'cd {self.posix}/host/opt && ln -s {self.posix}/host/opt/releases/{OLD} current && ln -s {self.posix}/host/opt/releases/{OLD}/bin/assetq {self.posix}/host/proc/100/exe')
        machine = hashlib.sha256(b"invented-machine\n").hexdigest()
        self.target = {"ssh_alias": "invented-alias", "service_account": "assetq", "service_uid": UID, "executable": "bin/assetq", "install_root": f"{self.posix}/host/opt", "unit": "assetq.service",
                       "port": 8740, "data_root": f"{self.posix}/host/data",
                       "expected": {"hostname": "invented-host", "machine_id_sha256": machine, "ssh_host_key_sha256": "SHA256:invented-key"},
                       "status_env_files": [f"{self.posix}/host/env/assetq.env"],
                       "min_install_free_bytes": 1000, "min_data_free_bytes": 1000, "health_timeout_seconds": 6}
        self.evidence = Evidence()
        patcher = patch.object(deploy, "gh_bytes", lambda path: self.evidence(path))
        patcher.start()
        self.addCleanup(patcher.stop)
        self.prompts = []
        self.answers = None
        patcher = patch.object(deploy, "ask", self.answer)
        patcher.start()
        self.addCleanup(patcher.stop)
        self.out = self.base / "plan"

    def answer(self, phrase, effect):
        self.prompts.append(phrase)
        if self.answers is not None and len(self.prompts) > self.answers:
            raise deploy.Refused(f"confirmation did not match: {phrase}")

    def sh_run(self, script):
        subprocess.run([self.sh, "-c", script], check=True)

    def call(self, *argv):
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = deploy.main(list(argv))
        self.stdout, self.stderr = out.getvalue(), err.getvalue()
        return code

    def plan(self, *extra, source=SOURCE, target=None):
        path = self.base / "target.json"
        path.write_text(json.dumps(target or self.target))
        return self.call("plan", "--target", str(path), "--source", source, "--out", str(self.out), *extra)

    def planned(self, *extra):
        self.assertEqual(self.plan(*extra), 0, self.stderr)
        self.reset_logs()
        return json.loads((self.out / "plan.json").read_text())

    def apply(self, plan, *extra):
        return self.call("apply", "--plan", str(self.out / "plan.json"), "--confirm-plan", plan["plan_sha256"], *extra)

    def rollback(self, plan, *extra):
        return self.call("rollback", "--plan", str(self.out / "plan.json"), "--confirm-plan", plan["plan_sha256"], *extra)

    def reset_logs(self):
        for name in ("ssh", "tools"):
            (self.base / "log" / name).write_text("")

    def link(self, name):
        path = self.host / "opt" / name
        return os.readlink(path).replace("\\", "/").split("/")[-1] if os.path.islink(path) else None

    def tools_log(self):
        path = self.base / "log/tools"
        return path.read_text() if path.exists() else ""

    def ssh_log(self):
        path = self.base / "log/ssh"
        return path.read_text() if path.exists() else ""

    def preserved(self):
        """Retained state, protected configuration and the preserved release, by content."""
        result = {}
        for directory in ["data", "env", f"opt/releases/{OLD}"]:
            for path in sorted((self.host / directory).rglob("*")):
                if path.is_file():
                    result[str(path.relative_to(self.host))] = sha(path.read_bytes())
        return result

    def snapshot(self):
        """Every host path and link. A no-effect command leaves this identical."""
        result = {}
        for path in sorted(self.host.rglob("*")):
            key = str(path.relative_to(self.host))
            result[key] = os.readlink(path) if os.path.islink(path) else sha(path.read_bytes()) if path.is_file() else "dir"
        return result

    def journal(self, suffix="journal"):
        return [json.loads(line) for line in (self.out / f"plan.json.{suffix}").read_text().splitlines()]


class Plan(Fixture):
    def test_plan_binds_all_evidence_without_host_effect(self):
        # A2: target, CI, source, archive, executable, UI, current release and rollback; no mutating command.
        before = self.snapshot()
        self.assertEqual(self.plan(), 0, self.stderr)
        plan = json.loads((self.out / "plan.json").read_text())
        name, archive = self.evidence.archive()
        self.assertEqual(plan["host"]["identity"]["hostname"], "invented-host")
        self.assertEqual((plan["ci"]["run_id"], plan["ci"]["event"], plan["ci"]["jobs"]),
                         (7, "merge_group", {"tests": "success", "candidate": "success"}))
        self.assertEqual(plan["source"], {"sha": SOURCE, "tree_sha": "tree-1"})
        self.assertEqual(plan["candidate"]["archive_sha256"], sha(archive))
        self.assertEqual(plan["candidate"]["binary_sha256"], sha(self.evidence.binary))
        self.assertEqual(plan["candidate"]["ui"], {"state": "bound", "files": {n: sha(d) for n, d in UI.items()}})
        self.assertEqual((plan["host"]["binding"]["current"], plan["rollback"]["release_id"]), (OLD, OLD))
        self.assertEqual((self.out / name).read_bytes(), archive)
        self.assertEqual(self.snapshot(), before)
        self.assert_read_only(self.ssh_log())
        self.assertFalse(re.search(r"systemctl --user (?!cat|show)", self.tools_log()), self.tools_log())
        for text in [self.ssh_log(), self.stdout, (self.out / "plan.json").read_text()]:
            self.assertNotIn(SECRET, text)

    def assert_read_only(self, scripts):
        scripts = "\n".join(line for line in scripts.splitlines() if not line.startswith("for tool in "))
        self.assertFalse(re.search(r"\b(mv|ln|rm|mkdir|touch|chmod|chown|tee|restart|stop|start|reload|enable|disable|migrate)\b", scripts), scripts)
        self.assertLessEqual(set(re.findall(r">\s*([^\s;]+)", scripts)), {"&2", "/dev/null"})

    def test_dry_run_verifies_again_without_effect(self):
        plan = self.planned()
        before = self.snapshot()
        self.assertEqual(self.apply(plan, "--dry-run"), 0, self.stderr)
        self.assertEqual(self.snapshot(), before)
        self.assert_read_only(self.ssh_log())
        self.assertFalse((self.out / "plan.json.journal").exists())
        self.assertEqual(self.prompts, [])

    def test_wrong_source_hash_or_ci_refuses_before_any_host_contact(self):
        def edit_job(e): e.jobs[1]["conclusion"] = "failure"
        def red(e): e.run["conclusion"] = "failure"
        def running(e): e.run.update(status="in_progress", conclusion=None)
        def push_only(e): e.run["event"] = "push"
        def absent(e): e.runs.clear()
        def other_workflow(e): e.run["path"] = ".github/workflows/other.yml"
        def expired(e): e.artifact_edit = {"expired": True}
        def other_run(e): e.artifact_edit = {"workflow_run": {"id": 8, "head_sha": SOURCE}}
        def artifact_digest(e): e.artifact_edit = {"digest": "sha256:" + "0" * 64}
        def no_digest(e): e.artifact_edit = {"digest": None}
        def unmerged(e): e.compare = "diverged"
        def outer_sums(e): e.outer_sums = "0" * 64 + "  x.tar.gz\n"
        def inner_sums(e): e.inner_sums = "0" * 64 + "  bin/assetq\n"
        def wrong_binary_digest(e): e.manifest["binary_sha256"] = "0" * 64
        def wrong_source(e): e.manifest["source_sha"] = OTHER
        def wrong_target(e): e.manifest["target"] = "aarch64-unknown-linux-gnu"
        def wrong_doc(e): e.doc_in_archive = b"# altered\n"
        def wrong_ui(e): e.manifest["embedded_ui"]["app.js"] = "0" * 64
        def missing_ui(e): del e.manifest["embedded_ui"]["app.js"]
        def no_ui_metadata(e): del e.manifest["embedded_ui"]
        def binary_lacks_ui(e):
            e.binary = executable(ui=False)
            e.manifest["binary_sha256"] = sha(e.binary)
        def no_schema(e): del e.manifest["schema"]
        def no_libc(e): e.manifest["libc"] = ["", ""]
        def not_zip(e): e.zip_override = b"not a zip"
        def traversal(e):
            e.extra_member = tarfile.TarInfo(f"assetq-0.1.0-{SOURCE[:12]}-{deploy.TARGET}/../escape")
        def symlink(e):
            e.extra_member = tarfile.TarInfo(f"assetq-0.1.0-{SOURCE[:12]}-{deploy.TARGET}/link")
            e.extra_member.type, e.extra_member.linkname = tarfile.SYMTYPE, "/etc/passwd"
        for case in [edit_job, red, running, push_only, absent, other_workflow, expired, other_run, artifact_digest, no_digest,
                     unmerged, outer_sums, inner_sums, wrong_binary_digest, wrong_source, wrong_target, wrong_doc, wrong_ui,
                     missing_ui, no_ui_metadata, binary_lacks_ui, no_schema, no_libc, not_zip, traversal, symlink]:
            with self.subTest(case.__name__):
                self.evidence = Evidence()
                case(self.evidence)
                self.reset_logs()
                self.assertEqual(self.plan(), deploy.EXIT_REFUSED, self.stdout)
                self.assertIn("refused:", self.stderr)
                self.assertFalse(self.out.exists())
                self.assertEqual(self.ssh_log(), "")
        self.evidence = Evidence()
        self.assertEqual(self.plan(source="abc"), deploy.EXIT_REFUSED)
        self.assertIn("full lowercase commit", self.stderr)

    def test_source_without_ui_needs_an_explicit_plan_record(self):
        self.evidence = Evidence(with_ui=False)
        self.assertEqual(self.plan(), deploy.EXIT_REFUSED)
        self.assertIn("--allow-no-ui", self.stderr)
        self.assertEqual(self.plan("--allow-no-ui"), 0, self.stderr)
        plan = json.loads((self.out / "plan.json").read_text())
        self.assertEqual(plan["candidate"]["ui"], {"state": "none-in-source", "files": {}})

    def test_ssh_or_identity_failure_refuses(self):
        (self.state / "ssh_fail").write_text("sh -c")
        self.assertEqual(self.plan(), deploy.EXIT_REFUSED)
        self.assertIn("Host key verification failed", self.stderr)
        self.assertFalse(self.out.exists())
        (self.state / "ssh_fail").unlink()
        for key, value in [("hostname", "other"), ("machine_id_sha256", "0" * 64), ("ssh_host_key_sha256", "SHA256:other")]:
            with self.subTest(key):
                target = json.loads(json.dumps(self.target))
                target["expected"][key] = value
                self.assertEqual(self.plan(target=target), deploy.EXIT_REFUSED)
                self.assertIn("identity", self.stderr)
                self.assertFalse(self.out.exists())

    def test_ssh_options_require_existing_known_host_key_and_no_prompt(self):
        recorded = []
        def record(argv, **kwargs):
            recorded.append(argv)
            return subprocess.CompletedProcess(argv, 0, b"", b"")
        with patch.object(deploy.subprocess, "run", record), patch.object(deploy, "SSH", ["ssh"]):
            deploy.Host(self.target).run("true")
        self.assertEqual(recorded[0][:7], ["ssh", "-o", "BatchMode=yes", "-o", "StrictHostKeyChecking=yes", "-o", "ConnectTimeout=10"])
        self.assertTrue(recorded[0][-1].startswith("sh -c "))

    def test_unverifiable_or_incompatible_host_refuses(self):
        def not_root(s): (s / "operator_uid").write_text("1000\n")
        def wrong_account(s): (s / "account_uid").write_text("1001\n")
        def low_install(s): (s / "free").write_text("10\n")
        def open_env(s): (s / "env_stat").write_text("root root 644 12 1700 5 8\n")
        def foreign_env(s): (s / "env_stat").write_text("other other 600 12 1700 5 8\n")
        def other_unit(s): (s / "exec").write_text("/usr/bin/other serve --listen 0.0.0.0:8740\n")
        def old_libc(s): (s / "libc").write_text("glibc 2.40\n")
        def stopped(s): (s / "active").write_text("failed\n")
        def bad_health(s): (s / "old_health").write_text('{"service":"up","database":"unknown: query failed"}\n')
        def bad_integrity(s): (s / "integrity").write_text('{"verified":false}')
        def active_jobs(s): (s / "ops").write_text(json.dumps({"jobs": {"by_phase": {"waiting": 1}, "uncertain_submissions": 0, "leased": 0}}))
        def uncertain_jobs(s): (s / "ops").write_text(json.dumps({"jobs": {"by_phase": {}, "uncertain_submissions": 1, "leased": 0}}))
        def no_status(s): (s / "ops").write_text("not json")
        for case in [not_root, wrong_account, low_install, open_env, foreign_env, other_unit, old_libc, stopped, bad_health, bad_integrity,
                     active_jobs, uncertain_jobs, no_status]:
            with self.subTest(case.__name__):
                saved = {p.name: p.read_bytes() for p in self.state.iterdir()}
                case(self.state)
                self.assertEqual(self.plan(), deploy.EXIT_REFUSED, self.stdout)
                self.assertFalse(self.out.exists())
                for name, data in saved.items():
                    (self.state / name).write_bytes(data)
        self.evidence = Evidence(schema=2)
        self.assertEqual(self.plan(), deploy.EXIT_REFUSED)
        self.assertIn("schema", self.stderr)
        self.evidence = Evidence(libc="2.50")
        self.assertEqual(self.plan(), deploy.EXIT_REFUSED)
        self.assertIn("glibc", self.stderr)

    def test_lifecycle_uses_only_the_named_user_manager(self):
        # The host runs a user unit. A system-level call or another runtime directory must not work.
        self.target["service_uid"] = 1001
        (self.state / "account_uid").write_text("1001\n")
        self.assertEqual(self.plan(), deploy.EXIT_REFUSED)
        self.assertIn("service user manager", self.stderr)
        self.assertFalse(self.out.exists())
        (self.state / "account_uid").write_text(f"{UID}\n")
        self.target["service_uid"] = UID
        self.assertEqual(deploy.Host(self.target).user_command("systemctl --user show"),
                         f"runuser -u assetq -- env XDG_RUNTIME_DIR=/run/user/{UID} systemctl --user show")
        self.assertEqual(deploy.subprocess.run([self.sh, "-c", "systemctl show"], capture_output=True).returncode, 1)

    def test_active_or_unknown_work_needs_a_reviewed_plan_record(self):
        self.target["status_env_files"] = []
        self.assertEqual(self.plan(), deploy.EXIT_REFUSED)
        self.assertIn("unknown", self.stderr)
        self.assertEqual(self.plan("--accept-active-work"), 0, self.stderr)
        plan = json.loads((self.out / "plan.json").read_text())
        self.assertEqual((plan["host"]["active_work"]["state"], plan["accepted_active_work"]), ("unknown", True))


class Layout(Fixture):
    def test_existing_flat_release_layout_and_resource_wrapper_are_preserved(self):
        # The host may run <release>/assetq behind a wrapper; the unit stays untouched and bound.
        (self.state / "executable").write_text("assetq\n")
        old = self.host / "opt/releases" / OLD
        shutil.copy(old / "bin/assetq", old / "assetq")
        (self.state / "exec").write_text(f"/usr/bin/prlimit --as=1 {self.posix}/host/opt/current/assetq serve --listen 127.0.0.1:8740\n")
        self.target["executable"] = "assetq"
        plan = self.planned()
        self.assertEqual(self.apply(plan), 0, self.stderr)
        self.assertEqual(self.link("current"), self.evidence.release)
        self.assertEqual((self.host / "opt/releases" / self.evidence.release / "assetq").read_bytes(), self.evidence.binary)
        self.assertEqual(self.rollback(plan), 0, self.stderr)
        self.assertEqual(self.link("current"), OLD)


class Stale(Fixture):
    def test_stale_plan_or_drift_refuses_before_effect(self):
        plan = self.planned()
        before = self.snapshot()
        path = self.out / "plan.json"
        original = path.read_text()

        future = deploy.now() + datetime.timedelta(seconds=deploy.PLAN_TTL_SECONDS + 1)
        with patch.object(deploy, "now", lambda: future):
            self.assertEqual(self.apply(plan), deploy.EXIT_REFUSED)
        self.assertIn("stale", self.stderr)

        self.assertEqual(self.call("apply", "--plan", str(path), "--confirm-plan", "0" * 64), deploy.EXIT_REFUSED)
        self.assertIn("reviewed plan digest", self.stderr)

        path.write_text(original.replace(self.evidence.release, "0" * 64))
        self.assertEqual(self.apply(plan), deploy.EXIT_REFUSED)
        self.assertIn("differs from its digest", self.stderr)
        path.write_text(original)

        archive = self.out / plan["candidate"]["archive_name"]
        saved = archive.read_bytes()
        archive.write_bytes(saved + b"x")
        self.assertEqual(self.apply(plan), deploy.EXIT_REFUSED)
        self.assertIn("differs from the plan", self.stderr)
        archive.write_bytes(saved)

        self.evidence.run["conclusion"] = "failure"
        self.assertEqual(self.apply(plan), deploy.EXIT_REFUSED)
        self.assertIn("not green", self.stderr)
        self.evidence.run["conclusion"] = "success"

        self.evidence.compare = "diverged"
        self.assertEqual(self.apply(plan), deploy.EXIT_REFUSED)
        self.assertIn("not contained in main", self.stderr)
        self.evidence.compare = "ahead"

        for name, value, reported in [("unit", "[Service]\nchanged\n", "unit_sha256"),
                                      ("env_stat", "root root 600 13 1800 5 8\n", "protected_stats")]:
            saved = (self.state / name).read_text()
            (self.state / name).write_text(value)
            self.assertEqual(self.apply(plan), deploy.EXIT_REFUSED)
            self.assertIn(reported, self.stderr)
            (self.state / name).write_text(saved)

        (self.state / "ops").write_text(json.dumps({"jobs": {"by_phase": {"waiting": 1}, "uncertain_submissions": 0, "leased": 0}}))
        self.assertEqual(self.apply(plan), deploy.EXIT_REFUSED)
        self.assertIn("active work", self.stderr)
        self.assertEqual(self.prompts, [])
        self.assertEqual(self.snapshot(), before)
        self.assertFalse((self.out / "plan.json.journal").exists())

    def test_current_release_changed_since_plan(self):
        plan = self.planned()
        other = self.host / "opt/releases/0.1.0-other"
        shutil.copytree(self.host / "opt/releases" / OLD, other)
        self.sh_run(f'cd {self.posix}/host/opt && rm current && ln -s releases/0.1.0-other current')
        self.assertEqual(self.apply(plan), deploy.EXIT_REFUSED)
        self.assertIn("current", self.stderr)


class Apply(Fixture):
    def test_apply_verifies_preserves_and_rolls_back_on_operator_request(self):
        plan = self.planned()
        before = self.preserved()
        self.assertEqual(self.apply(plan), 0, self.stderr)
        self.assertEqual(self.prompts, [a["phrase"] for a in plan["actions"]])
        self.assertEqual((self.link("current"), self.link("previous")), (self.evidence.release, OLD))
        self.assertTrue((self.host / "opt/releases" / self.evidence.release / "healthy").exists())
        self.assertFalse((self.host / "opt/deploy.lock").exists())
        self.assertFalse((self.host / "opt/releases" / f"{self.evidence.release}.tmp").exists())
        self.assertEqual(self.preserved(), before)
        self.assertEqual((self.host / "opt/releases" / self.evidence.release / "bin/assetq").read_bytes(), self.evidence.binary)
        log = self.tools_log()
        self.assertEqual(log.count("systemctl --user restart"), 1)
        self.assertEqual(log.count("migrate candidate"), 1)
        self.assertLess(log.index("migrate candidate"), log.index("systemctl --user restart"))
        self.assertIn("--property=EnvironmentFile=", log)
        self.assertNotIn("daemon-reload", log)
        steps = [(j["step"], j["status"]) for j in self.journal()]
        self.assertEqual(steps, [("plan", "accepted"), ("prepare", "done"), ("migrate", "intent"), ("migrate", "done"),
                                 ("activate", "intent"), ("activate", "verified")])
        self.assertIn("deployed and verified on this host", self.stdout)
        for text in [self.ssh_log(), self.stdout, (self.out / "plan.json.journal").read_text()]:
            self.assertNotIn(SECRET, text)
        self.assertEqual(self.apply(plan), deploy.EXIT_REFUSED)
        self.assertIn("already current", self.stderr)
        self.prompts.clear()
        self.reset_logs()
        before_rollback = self.snapshot()
        self.assertEqual(self.rollback(plan, "--dry-run"), 0, self.stderr)
        self.assertEqual(self.snapshot(), before_rollback)
        self.assertEqual(self.prompts, [])
        self.assertEqual(self.rollback(plan), 0, self.stderr)
        self.assertEqual(self.prompts, [f"rollback assetq.service on invented-host to {OLD[:12]}"])
        self.assertEqual((self.link("current"), self.link("previous")), (OLD, None))
        self.assertEqual(self.tools_log().count("systemctl --user restart"), 1)
        self.assertEqual(self.preserved(), before)
        self.assertTrue((self.host / "opt/releases" / self.evidence.release / "bin/assetq").exists())
        self.assertEqual(self.rollback(plan), deploy.EXIT_REFUSED)
        self.assertIn("rollback does not apply", self.stderr)

    def test_failed_verification_rolls_back_to_the_preserved_release(self):
        cases = [("health", dict(health="fail")), ("crash", dict(start=1)), ("integrity", dict(integrity="integrity_low"))]
        for name, options in cases:
            with self.subTest(name):
                self.doCleanups()
                self.setUp()
                self.evidence = Evidence(binary=executable(**options))
                plan = self.planned()
                before = self.preserved()
                self.assertEqual(self.apply(plan), deploy.EXIT_ROLLED_BACK, self.stderr)
                self.assertEqual(self.link("current"), OLD)
                self.assertIsNone(self.link("previous"))
                self.assertEqual(self.preserved(), before)
                self.assertFalse((self.host / "opt/deploy.lock").exists())
                self.assertFalse((self.host / "opt/releases" / self.evidence.release / "healthy").exists())
                self.assertTrue((self.host / "opt/releases" / self.evidence.release / "bin/assetq").exists())
                self.assertEqual(self.tools_log().count("systemctl --user restart"), 2)
                pid = (self.state / "pid").read_text().strip()
                self.assertEqual(os.readlink(self.host / f"proc/{pid}/exe").replace("\\", "/").split("/")[-3], OLD)
                steps = [(j["step"], j["status"]) for j in self.journal()]
                self.assertIn(("verify", "failed"), steps)
                self.assertEqual(steps[-1], ("rollback", "done"))
                self.assertIn('"result": "rolled back"', self.stdout)
                self.assertEqual(self.apply(plan), deploy.EXIT_REFUSED)
                self.assertIn("already used", self.stderr)

    def test_changed_service_configuration_after_restart_rolls_back(self):
        # A3: resource guards live in the unit; drift during activation is never accepted.
        plan = self.planned()
        real = deploy.restart

        def restart_with_drift(host):
            (self.state / "unit").write_text("[Service]\nLimitAS=infinity\n")
            return real(host)
        # The rollback restart then observes drift only through its own process checks.
        with patch.object(deploy, "restart", restart_with_drift):
            self.assertEqual(self.apply(plan), deploy.EXIT_ROLLED_BACK, self.stderr)
        self.assertIn("unit_sha256", self.journal()[-3]["reason"] + self.stdout)
        self.assertEqual(self.link("current"), OLD)

    def test_lost_ssh_during_activation_is_uncertain_and_blocks_a_second_apply(self):
        plan = self.planned()
        (self.state / "ssh_fail").write_text("systemctl --user restart")
        self.assertEqual(self.apply(plan), deploy.EXIT_UNCERTAIN)
        self.assertIn("activation outcome unknown", self.stderr)
        self.assertEqual(self.link("current"), self.evidence.release)
        self.assertTrue((self.host / "opt/deploy.lock").is_dir())
        self.assertEqual(self.journal()[-1]["status"], "uncertain")
        self.assertNotIn("deployed and verified", self.stdout)
        (self.state / "ssh_fail").unlink()
        self.assertEqual(self.apply(plan), deploy.EXIT_REFUSED)
        self.assertEqual(self.tools_log().count("systemctl --user restart"), 0)

    def test_lost_ssh_before_activation_is_uncertain_and_leaves_current_in_place(self):
        for lost in ("mkdir deploy.lock", "archive.tar.gz"):
            with self.subTest(lost):
                self.doCleanups()
                self.setUp()
                plan = self.planned()
                (self.state / "ssh_fail").write_text(lost)
                self.assertEqual(self.apply(plan), deploy.EXIT_UNCERTAIN)
                self.assertEqual(self.link("current"), OLD)
                self.assertNotIn("systemctl --user restart", self.tools_log())
                self.assertEqual((self.state / "pid").read_text().strip(), "100")

    def test_unverified_rollback_is_uncertain_not_rolled_back(self):
        self.evidence = Evidence(binary=executable(health="fail"))
        plan = self.planned()
        real = deploy.restart

        def restart_breaking_old(host):
            (self.state / "old_health").write_text("not json\n")
            return real(host)
        with patch.object(deploy, "restart", restart_breaking_old):
            self.assertEqual(self.apply(plan), deploy.EXIT_UNCERTAIN)
        self.assertIn("rollback did not verify", self.stderr)
        self.assertTrue((self.host / "opt/deploy.lock").is_dir())
        self.assertEqual(self.journal()[-1]["status"], "uncertain")

    def test_failed_preparation_or_migrate_leaves_the_service_unchanged(self):
        cases = [("selftest", dict(selftest=1), "preparation failed"), ("migrate", dict(migrate=1), "migrate failed")]
        for name, options, reported in cases:
            with self.subTest(name):
                self.doCleanups()
                self.setUp()
                self.evidence = Evidence(binary=executable(**options))
                plan = self.planned()
                before = self.preserved()
                self.assertEqual(self.apply(plan), deploy.EXIT_REFUSED, self.stdout)
                self.assertIn(reported, self.stderr)
                self.assertEqual(self.link("current"), OLD)
                self.assertNotIn("systemctl --user restart", self.tools_log())
                self.assertFalse((self.host / "opt/deploy.lock").exists())
                self.assertFalse((self.host / "opt/releases" / f"{self.evidence.release}.tmp").exists())
                self.assertEqual(self.preserved(), before)
                self.assertEqual((self.state / "pid").read_text().strip(), "100")

    def test_existing_release_bytes_are_reused_or_preserved(self):
        plan = self.planned()
        release = self.host / "opt/releases" / self.evidence.release
        (release / "bin").mkdir(parents=True)
        (release / "bin/assetq").write_bytes(b"different bytes\n")
        (release / "candidate.json").write_text("{}")
        self.assertEqual(self.apply(plan), deploy.EXIT_REFUSED)
        self.assertIn("different bytes", self.stderr)
        self.assertEqual((release / "bin/assetq").read_bytes(), b"different bytes\n")
        self.assertEqual(self.link("current"), OLD)

    def test_held_lock_refuses_and_is_not_removed(self):
        plan = self.planned()
        (self.host / "opt/deploy.lock").mkdir()
        self.assertEqual(self.apply(plan), deploy.EXIT_REFUSED)
        self.assertIn("deploy.lock", self.stderr)
        self.assertTrue((self.host / "opt/deploy.lock").is_dir())
        self.assertEqual(self.link("current"), OLD)

    def test_each_declined_action_stops_there(self):
        for answered, restarted, current in [(0, False, OLD), (1, False, OLD), (2, False, OLD)]:
            with self.subTest(answered=answered):
                self.doCleanups()
                self.setUp()
                plan = self.planned()
                self.answers = answered
                self.assertEqual(self.apply(plan), deploy.EXIT_REFUSED)
                self.assertIn("confirmation did not match", self.stderr)
                self.assertEqual(self.link("current"), current)
                self.assertEqual("systemctl --user restart" in self.tools_log(), restarted)
                self.assertEqual("migrate candidate" in self.tools_log(), answered == 2)
                self.assertFalse((self.host / "opt/deploy.lock").exists())
                self.assertEqual((self.host / "opt/releases" / self.evidence.release).exists(), answered >= 1)

    def test_live_actions_refuse_automation_and_non_interactive_callers(self):
        plan = self.planned()
        before = self.snapshot()
        for name in ("CI", "GITHUB_ACTIONS", "TASKQ_TASK"):
            with self.subTest(name), patch.dict(os.environ, {name: "true"}):
                self.assertEqual(self.apply(plan), deploy.EXIT_REFUSED)
                self.assertIn(name, self.stderr)
                self.assertEqual(self.rollback(plan), deploy.EXIT_REFUSED)
        self.assertEqual(self.prompts, [])
        self.assertEqual(self.snapshot(), before)
        with patch.object(deploy.sys, "stdin", io.StringIO("")):
            with self.assertRaisesRegex(deploy.Refused, "interactive terminal"):
                ORIGINAL_ASK("invented phrase", "invented effect")


ORIGINAL_ASK = deploy.ask


if __name__ == "__main__":
    unittest.main()
