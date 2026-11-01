"""Bootstrap contract gate; add affected implementation checks with the first code."""
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
PIN = "45ff029915bc8e0e53b4523217de636183aa6ad9"
SPECQ = ROOT / ".specq"


def run(*args, cwd=ROOT):
    return subprocess.run(args, cwd=cwd, check=True, timeout=120, text=True,
                          stdout=subprocess.PIPE, stderr=subprocess.STDOUT).stdout


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
    print("Contract and whitespace checks passed; implementation is not qualified.")


if __name__ == "__main__":
    try:
        main()
    except (RuntimeError, subprocess.SubprocessError) as error:
        print(getattr(error, "stdout", None) or str(error), file=sys.stderr)
        raise SystemExit(1)
