"""R3/R4/R5/R6: the bounded driver never replaces a failed or interrupted job."""
from pathlib import Path
import hashlib
import json
import re
import tempfile
from unittest.mock import patch
import unittest

from qualify_image import TEXT, exercise, main, swatch
from import_retained_image import import_request, main as retained_main


class QualificationDriver(unittest.TestCase):
    def test_worker_role_refuses_before_any_files_or_host_effects(self):
        with tempfile.TemporaryDirectory() as root:
            directory = Path(root) / "new-run"
            args = ["qualify_image.py", "--binary", "absent", "--adapter", "absent", "--workspace", root, "--directory", str(directory)]
            with patch("sys.argv", args), patch.dict("os.environ", {"TASKQ_ATTEMPT": "invented-worker"}):
                with self.assertRaisesRegex(RuntimeError, "worker cannot control"):
                    main()
            self.assertFalse(directory.exists())

    def test_retained_import_keeps_source_and_refuses_worker_before_effects(self):
        source={"id":"invented-source","request":{"text":"original text","attachments":["invented-attachment"],"constraints":{"max_width":1024}}}
        body=import_request(source)
        self.assertEqual(body,import_request(source))
        self.assertEqual(body["operation"],"image.import_native")
        self.assertEqual(body["attachments"],source["request"]["attachments"])
        self.assertEqual(body["constraints"],{"source_job":"invented-source"})
        self.assertEqual(source["request"]["constraints"],{"max_width":1024})
        with tempfile.TemporaryDirectory() as root:
            new=Path(root)/"absent"
            args=["import_retained_image.py","--binary","absent","--workspace",root,"--directory",str(new),"--sha256","0"*64]
            with patch("sys.argv",args),patch.dict("os.environ",{"TASKQ_ATTEMPT":"invented-worker"}):
                with self.assertRaisesRegex(RuntimeError,"worker cannot reconcile"):
                    retained_main()
            self.assertFalse(new.exists())

    def test_brief_matches_scoped_runner_and_input_is_labeled_separately(self):
        source = (Path(__file__).resolve().parents[1] / "src/image_worker.rs").read_text()
        self.assertEqual(TEXT, json.loads(re.search(r'pub const QUALIFICATION_TEXT: &str = (".*?");', source).group(1)))
        self.assertEqual(swatch()[:8], b"\x89PNG\r\n\x1a\n")

    def test_one_job_duplicate_restart_retrieval_and_terminal_refusal(self):
        for outcome in ["succeeded", "failed", "corrupt"]:
            with self.subTest(outcome=outcome), tempfile.TemporaryDirectory() as root:
                directory = Path(root)
                spool = directory / "store/tmp/image-worker/invented-op"
                spool.mkdir(parents=True)
                (spool / "native.jsonl").write_text('{"message":{"operation":"input"}}\n')
                submits, restarts, reads = [], [], []
                payload = b"invented fixture bytes, never a real generated image"
                digest = hashlib.sha256(payload).hexdigest()
                def api(path, body=None, download=False):
                    if path == "/v1/jobs":
                        submits.append(body)
                        return {"created": len(submits) == 1, "job": {"id": "invented-job"}}
                    if path.endswith("/events"):
                        return {"events": []}
                    if path.endswith("/result"):
                        return {"manifest": {"items": [{"object_id": str(i), "import_key": f"output-{i}", "sha256": digest} for i in range(3)]}}
                    if path.endswith("/content"):
                        return payload, "corrupt" if outcome == "corrupt" else digest
                    if path.startswith("/v1/objects/"):
                        return {"sha256": digest}
                    reads.append(path)
                    return {"phase": "waiting" if len(reads) == 1 else ("failed" if outcome == "failed" else "succeeded"),
                            "provider_operation": "invented-op", "submissions": 1}
                if outcome == "succeeded":
                    self.assertEqual(exercise(api, restarts.append, directory, "invented-attachment", wait=lambda _: None), "invented-job")
                    self.assertEqual((directory / "output-0.download").read_bytes(), payload)
                else:
                    with self.assertRaises((RuntimeError, AssertionError)):
                        exercise(api, restarts.append, directory, "invented-attachment", wait=lambda _: None)
                self.assertEqual(restarts, ["invented-job", "invented-job"])
                self.assertEqual(len(submits), 3)  # All identical; exactly one durable identity.
                self.assertTrue(all(body == submits[0] for body in submits))


if __name__ == "__main__":
    unittest.main()
