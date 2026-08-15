from __future__ import annotations

import copy
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "code" / "scripts"))

import validate_d18q_channel_key_grant_conformance as conformance  # noqa: E402


class D18QChannelKeyGrantConformanceTests(unittest.TestCase):
    def setUp(self) -> None:
        _, self.manifest = conformance.load_manifest()

    def test_repository_contract_is_complete(self) -> None:
        document = conformance.validate_repository()
        self.assertEqual(
            "D18Q-channel-key-grant-fixtures-v1", document["fixture_format"]
        )
        self.assertEqual(
            conformance.EXPECTED_LANE_IDS,
            {lane.lane_id for lane in conformance.LANES},
        )

    def test_lane_roster_rejects_missing_duplicate_and_reused_paths(self) -> None:
        with self.assertRaisesRegex(
            conformance.D18QConformanceError, "exactly the supported six"
        ):
            conformance.validate_lane_roster(conformance.LANES[:-1])
        duplicated = conformance.LANES[:-1] + (conformance.LANES[0],)
        with self.assertRaisesRegex(
            conformance.D18QConformanceError, "exactly the supported six"
        ):
            conformance.validate_lane_roster(duplicated)
        reused = conformance.LANES[:-1] + (
            conformance.Lane(
                "elixir",
                conformance.LANES[0].package_root,
                conformance.LANES[0].consumer_test,
            ),
        )
        with self.assertRaisesRegex(
            conformance.D18QConformanceError, "paths must be unique"
        ):
            conformance.validate_lane_roster(reused)

    def test_manifest_rejects_missing_case_and_transition_drift(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["opening_negative_cases"] = changed["opening_negative_cases"][:-1]
        with self.assertRaisesRegex(
            conformance.D18QConformanceError, "opening negatives roster"
        ):
            conformance.validate_manifest(changed)

        changed = copy.deepcopy(self.manifest)
        changed["receiver_state_trace"]["steps"][4]["retained_epochs"] = ["3"]
        with self.assertRaisesRegex(
            conformance.D18QConformanceError, "transition trace"
        ):
            conformance.validate_manifest(changed)

    def test_manifest_rejects_secret_capability_and_error_drift(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["secret_erasure_capabilities"][-1] = "unknown"
        with self.assertRaisesRegex(
            conformance.D18QConformanceError, "capability vocabulary"
        ):
            conformance.validate_manifest(changed)

        changed = copy.deepcopy(self.manifest)
        changed["stable_error_codes"][-1] = "unknown"
        with self.assertRaisesRegex(conformance.D18QConformanceError, "error roster"):
            conformance.validate_manifest(changed)

    def test_generator_blob_hash_matches_manifest(self) -> None:
        generator = conformance.GENERATOR_PATH.read_bytes()
        self.assertEqual(
            self.manifest["generator_blob_sha1"],
            conformance.git_blob_sha1(generator),
        )

    def test_build_lines_run_independently_from_the_package_root(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package = root / "package"
            package.mkdir()
            (package / "BUILD").write_text(
                "cd ../dependency && install-tool\nrun-package-tests\n",
                encoding="utf-8",
            )
            lane = conformance.Lane("test", "package", "consumer")
            with mock.patch.object(conformance, "_run") as run:
                conformance.run_lane(lane, root)
            self.assertEqual(
                [
                    mock.call(
                        ["bash", "-c", "cd ../dependency && install-tool"],
                        package,
                        "D18Q test lane command 1/2",
                    ),
                    mock.call(
                        ["bash", "-c", "run-package-tests"],
                        package,
                        "D18Q test lane command 2/2",
                    ),
                ],
                run.call_args_list,
            )

    def test_ci_workflow_has_stable_job_and_aggregate_dependency(self) -> None:
        workflow = (ROOT / ".github" / "workflows" / "ci.yml").read_text(
            encoding="utf-8"
        )
        self.assertIn("d18q-channel-key-grant-conformance:", workflow)
        self.assertIn("D18Q channel-key grant conformance", workflow)
        self.assertIn(
            "python3 code/scripts/validate_d18q_channel_key_grant_conformance.py",
            workflow,
        )
        self.assertIn("D18Q_RESULT:", workflow)


if __name__ == "__main__":
    unittest.main()
