from __future__ import annotations

import copy
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "code" / "scripts"))

import validate_d18f_message_conformance as conformance  # noqa: E402


class D18FMessageConformanceTests(unittest.TestCase):
    def setUp(self) -> None:
        _, self.manifest = conformance.load_manifest()

    def test_repository_contract_is_complete(self) -> None:
        document = conformance.validate_repository()
        self.assertEqual("D18F-message-fixtures-v1", document["fixture_format"])
        self.assertEqual(
            conformance.EXPECTED_LANE_IDS, {lane.lane_id for lane in conformance.LANES}
        )

    def test_lane_roster_rejects_missing_and_duplicate_languages(self) -> None:
        with self.assertRaisesRegex(
            conformance.D18FConformanceError, "exactly the supported six"
        ):
            conformance.validate_lane_roster(conformance.LANES[:-1])
        duplicated = conformance.LANES[:-1] + (conformance.LANES[0],)
        with self.assertRaisesRegex(
            conformance.D18FConformanceError, "exactly the supported six"
        ):
            conformance.validate_lane_roster(duplicated)

    def test_manifest_rejects_missing_rich_and_stream_cases(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["positive_cases"] = [
            case
            for case in changed["positive_cases"]
            if case["name"] != "multipart-related"
        ]
        with self.assertRaisesRegex(
            conformance.D18FConformanceError, "positive fixture roster"
        ):
            conformance.validate_manifest(changed)

    def test_manifest_rejects_stable_error_drift(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["binary_negative_cases"][0]["expected_error"] = "invalid_field"
        with self.assertRaisesRegex(
            conformance.D18FConformanceError, "semantics drifted"
        ):
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
                        "D18F test lane command 1/2",
                    ),
                    mock.call(
                        ["bash", "-c", "run-package-tests"],
                        package,
                        "D18F test lane command 2/2",
                    ),
                ],
                run.call_args_list,
            )


if __name__ == "__main__":
    unittest.main()
