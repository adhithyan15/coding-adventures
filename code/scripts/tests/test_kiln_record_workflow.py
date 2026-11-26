"""Guards for the Kiln record workflow file (it lives on the wip branch until the selector lands)."""

import pathlib
import unittest

import yaml

WORKFLOW = pathlib.Path(__file__).resolve().parents[3] / ".github/workflows/kiln-record.yml"


class RecordWorkflowTest(unittest.TestCase):
    def setUp(self):
        self.doc = yaml.safe_load(WORKFLOW.read_text())

    def test_shard_count_equals_matrix_length(self):
        # The merge job requires exactly SHARD_COUNT artifacts, and the matrix is static.
        count = int(self.doc["env"]["SHARD_COUNT"])
        matrix = self.doc["jobs"]["record"]["strategy"]["matrix"]["shard"]
        self.assertEqual(count, len(matrix))
        self.assertEqual(sorted(matrix), list(range(count)))

    def test_no_error_annotation_inside_the_stopped_recording_step(self):
        steps = self.doc["jobs"]["record"]["steps"]
        rec = next(s for s in steps if s.get("id") == "record")
        self.assertNotIn("::error::", rec["run"])


if __name__ == "__main__":
    unittest.main()
