"""R8 risk: a modified cached rule checker must not execute as the pinned gate."""
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import check


class GateTrust(unittest.TestCase):
    def test_unknown_impact_keeps_implementation_checks(self):
        self.assertFalse(check.needs_rust(["assetq.md", "docs/service.md"]))
        for paths in [[], ["Cargo.lock"], ["src/api.rs", "README.md"], ["scripts/check.py"], [".github/workflows/tests.yml"]]:
            self.assertTrue(check.needs_rust(paths), paths)

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
