"""R8 risk: a modified cached rule checker must not execute as the pinned gate."""
from pathlib import Path
import tempfile
import queue
import threading
import sys
import unittest
from unittest.mock import patch

import check


class GateTrust(unittest.TestCase):
    def test_background_child_cannot_hold_command_output_open(self):
        # R8: pg_ctl exits while its background server retains stdout.
        with tempfile.TemporaryDirectory() as directory:
            release = Path(directory) / "release"
            child = "import pathlib,sys,time; p=pathlib.Path(sys.argv[1]); end=time.monotonic()+10\nwhile not p.exists() and time.monotonic()<end: time.sleep(.01)"
            parent = "import subprocess,sys; subprocess.Popen([sys.executable,'-c',sys.argv[1],sys.argv[2]],close_fds=False); print('parent finished')"
            result = queue.Queue()
            def run_parent():
                try:
                    result.put(check.run(sys.executable, "-c", parent, child, str(release), timeout=5))
                except Exception as error:
                    result.put(error)
            thread = threading.Thread(target=run_parent, daemon=True)
            thread.start()
            try:
                self.assertEqual(result.get(timeout=3).strip(), "parent finished")
            finally:
                release.touch()
                thread.join(timeout=10)
            self.assertFalse(thread.is_alive())

    def test_unknown_impact_keeps_implementation_checks(self):
        self.assertFalse(check.needs_rust(["assetq.md", "docs/service.md"]))
        for paths in [[], ["Cargo.lock"], ["src/api.rs", "README.md"], ["scripts/check.py"], [".github/workflows/tests.yml"]]:
            self.assertTrue(check.needs_rust(paths), paths)

    def test_final_docs_commit_cannot_hide_branch_implementation(self):
        # R8: select the complete candidate, not only its final commit.
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            check.run("git", "init", str(root))
            def commit(name):
                (root / name).write_text("invented fixture\n")
                check.run("git", "add", name, cwd=root)
                check.run("git", "-c", "user.name=Fixture", "-c",
                          "user.email=fixture@example.invalid", "commit", "-m", "fixture", cwd=root)
            commit("base.md")
            check.run("git", "update-ref", "refs/remotes/origin/main", "HEAD", cwd=root)
            original_run = check.run
            commit("docs.md")
            with patch.object(check, "run", side_effect=lambda *args: original_run(*args, cwd=root)):
                self.assertFalse(check.needs_rust(check.changed_paths()))
            commit("Cargo.lock")
            commit("evidence.md")
            with patch.object(check, "run", side_effect=lambda *args: original_run(*args, cwd=root)):
                self.assertTrue(check.needs_rust(check.changed_paths()))
                self.assertIn("Cargo.lock", check.changed_paths())
            check.run("git", "update-ref", "refs/remotes/origin/main", "HEAD", cwd=root)
            with patch.object(check, "run", side_effect=lambda *args: original_run(*args, cwd=root)):
                self.assertEqual(check.changed_paths(), ["evidence.md"])
            (root / "staged.rs").write_text("invented fixture\n")
            check.run("git", "add", "staged.rs", cwd=root)
            (root / "untracked.rs").write_text("invented fixture\n")
            with patch.object(check, "run", side_effect=lambda *args: original_run(*args, cwd=root)):
                self.assertTrue({"staged.rs", "untracked.rs"}.issubset(check.changed_paths()))
            check.run("git", "update-ref", "-d", "refs/remotes/origin/main", cwd=root)
            with patch.object(check, "run", side_effect=lambda *args: original_run(*args, cwd=root)):
                self.assertIn("unknown-base", check.changed_paths())

    def test_modified_checker_is_refused_before_execution(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            rules = root / ".specq"
            rules.mkdir()
            check.run("git", "init", str(rules))
            checker = rules / "checker.py"
            checker.write_text("print('original')\n")
            check.run("git", "add", ".", cwd=rules)
            check.run("git", "-c", "user.name=Fixture", "-c",
                      "user.email=fixture@example.invalid", "commit", "-m", "fixture", cwd=rules)
            pin = check.run("git", "rev-parse", "HEAD", cwd=rules).strip()
            checker.write_text("raise SystemExit('modified checker executed')\n")
            original_run = check.run

            def bounded_run(*args, **kwargs):
                if args == (check.sys.executable, "scripts/test_check.py"):
                    return ""
                return original_run(*args, **kwargs)

            with patch.multiple(check, ROOT=root, SPECQ=rules, PIN=pin), \
                    patch.object(check, "run", side_effect=bounded_run):
                with self.assertRaisesRegex(RuntimeError, "pinned clean checkout"):
                    check.main()


if __name__ == "__main__":
    unittest.main()
