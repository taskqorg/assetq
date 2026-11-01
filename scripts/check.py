"""Canonical gate (R8): contract lint, whitespace, then deterministic Rust checks
against a disposable PostgreSQL cluster. Native, provider and storage
qualification stay separate and are not claimed by this gate."""
from contextlib import contextmanager
from pathlib import Path
import glob
import os
import shutil
import socket
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
PIN = "45ff029915bc8e0e53b4523217de636183aa6ad9"
SPECQ = ROOT / ".specq"


def run(*args, cwd=ROOT, timeout=120, env=None):
    # A Windows PostgreSQL child can inherit stdout past pg_ctl's exit.
    # A file keeps that handle from turning completion into an endless EOF wait.
    with tempfile.TemporaryFile(mode="w+", encoding="utf-8") as output:
        result = subprocess.run(args, cwd=cwd, timeout=timeout, text=True, env=env,
                                stdout=output, stderr=subprocess.STDOUT,
                                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0)
        output.seek(0)
        text = output.read()
        if result.returncode:
            raise subprocess.CalledProcessError(result.returncode, args, output=text)
        return text


def pg_bin():
    candidates = [os.environ.get("ASSETQ_PG_BIN")]
    candidates += sorted(glob.glob("/usr/lib/postgresql/*/bin"), reverse=True)
    candidates += sorted(glob.glob("/opt/homebrew/opt/postgresql@*/bin"), reverse=True)
    which = shutil.which("initdb")
    candidates.append(which and str(Path(which).parent))
    for directory in candidates:
        suffix = ".exe" if os.name == "nt" else ""
        if directory and (Path(directory) / f"initdb{suffix}").exists() and (Path(directory) / f"pg_ctl{suffix}").exists():
            return Path(directory)
    raise RuntimeError("PostgreSQL server binaries (initdb, pg_ctl) not found; set ASSETQ_PG_BIN")


@contextmanager
def postgres_cluster():
    """A throwaway cluster on a private socket; fsync is off because the
    fixture is disposable. Yields a libpq keyword string without dbname."""
    binaries = pg_bin()
    base = Path(tempfile.mkdtemp(prefix="aq-pg-"))
    data = base / "data"
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        port = probe.getsockname()[1]
    env = dict(os.environ, LC_ALL="C", LANG="C")
    try:
        run(str(binaries / "initdb"), "-D", str(data), "-U", "assetq", "--auth=trust", "-E", "UTF8",
            "--no-locale", env=env)
        host_options = "-h 127.0.0.1" if os.name == "nt" else f"-k {base} -c listen_addresses=''"
        options = f"{host_options} -p {port} -c fsync=off -c max_connections=300"
        try:
            run(str(binaries / "pg_ctl"), "-D", str(data), "-l", str(base / "log"), "-w", "-o", options,
                "start", env=env)
        except subprocess.CalledProcessError as error:
            log = (base / "log").read_text() if (base / "log").exists() else ""
            raise RuntimeError(f"PostgreSQL fixture did not start: {error.stdout}{log}") from error
        try:
            yield f"host={'127.0.0.1' if os.name == 'nt' else base} port={port} user=assetq"
        finally:
            run(str(binaries / "pg_ctl"), "-D", str(data), "-m", "immediate", "stop", env=env)
    finally:
        shutil.rmtree(base, ignore_errors=True)


def rust_checks(name=None):
    print(run("cargo", "fmt", "--check"), end="")
    with postgres_cluster() as pg:
        env = dict(os.environ, ASSETQ_TEST_PG=pg, ASSETQ_TEST_PG_BIN=str(pg_bin()))
        if os.name == "nt":
            # Keep unrelated native database fixtures from starving each other;
            # concurrency cases still launch their own competing callers.
            env["RUST_TEST_THREADS"] = "1"
        print(run("cargo", "test", "--locked", *([name] if name else []), timeout=480, env=env), end="")
    print(f"Selected Rust checks passed (filter: {name or 'all'}); fixture providers do not qualify real generation.")


def needs_rust(paths):
    # An unknown file type is implementation impact, never silently docs-only.
    return not paths or any(not p.endswith(".md") for p in paths)


def main():
    print(run(sys.executable, "scripts/test_check.py"), end="")
    if not SPECQ.exists():
        SPECQ.mkdir()
        run("git", "init", str(SPECQ))
        run("git", "-C", str(SPECQ), "remote", "add", "origin",
            "https://github.com/taskqorg/specq")
    if not (SPECQ / ".git").is_dir():
        raise RuntimeError(".specq is not the owned gate checkout; preserve and inspect it")
    try:
        actual = run("git", "-C", str(SPECQ), "rev-parse", "HEAD").strip()
    except subprocess.CalledProcessError:
        run("git", "-C", str(SPECQ), "fetch", "--depth=1", "origin", PIN)
        run("git", "-C", str(SPECQ), "checkout", "--detach", PIN)
        actual = run("git", "-C", str(SPECQ), "rev-parse", "HEAD").strip()
    if actual != PIN or run("git", "-C", str(SPECQ), "status", "--porcelain").strip():
        raise RuntimeError(".specq differs from the pinned clean checkout; inspect it")
    print(run(sys.executable, str(SPECQ / "engineering/scripts/contract_lint.py"),
              "assetq.md"), end="")
    print(run("git", "diff", "--check"), end="")
    print(run("git", "diff", "--cached", "--check"), end="")
    print("Contract and whitespace checks passed.")
    try:
        changed = run("git", "diff", "--name-only", "HEAD^", "HEAD").splitlines()
    except subprocess.CalledProcessError:
        changed = ["unknown-base"]
    changed += run("git", "diff", "--name-only", "HEAD").splitlines()
    changed += run("git", "ls-files", "--others", "--exclude-standard").splitlines()
    if needs_rust(changed):
        rust_checks()
    else:
        print("Docs-only candidate: Rust implementation unchanged.")


if __name__ == "__main__":
    try:
        if len(sys.argv) == 3 and sys.argv[1] == "--test":
            rust_checks(sys.argv[2])
        elif len(sys.argv) == 1:
            main()
        else:
            raise RuntimeError("usage: python3 scripts/check.py [--test NAME]")
    except (RuntimeError, subprocess.SubprocessError) as error:
        print(getattr(error, "stdout", None) or str(error), file=sys.stderr)
        raise SystemExit(1)
