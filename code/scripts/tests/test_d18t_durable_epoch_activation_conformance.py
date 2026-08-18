from __future__ import annotations

import copy
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "code" / "scripts"))

import validate_d18t_durable_epoch_activation_conformance as conformance  # noqa: E402


class D18TDurableEpochActivationConformanceTests(unittest.TestCase):
    """The gate must reject drift, not merely accept the current tree.

    Every test here mutates a valid manifest in one specific way and requires a
    rejection. A gate that only ever ran against a correct repository would pass
    forever without proving it can fail.
    """

    def setUp(self) -> None:
        _, self.manifest = conformance.load_manifest()

    def test_repository_contract_is_complete(self) -> None:
        document = conformance.validate_repository()
        self.assertEqual(
            "D18T-durable-epoch-activation-fixtures-v1", document["fixture_format"]
        )
        self.assertEqual(
            conformance.EXPECTED_LANE_IDS,
            {lane.lane_id for lane in conformance.LANES},
        )

    def test_lane_roster_rejects_missing_duplicate_and_reused_paths(self) -> None:
        with self.assertRaisesRegex(
            conformance.D18TConformanceError, "exactly the supported six"
        ):
            conformance.validate_lane_roster(conformance.LANES[:-1])

        duplicated = conformance.LANES[:-1] + (conformance.LANES[0],)
        with self.assertRaisesRegex(
            conformance.D18TConformanceError, "exactly the supported six"
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
            conformance.D18TConformanceError, "paths must be unique"
        ):
            conformance.validate_lane_roster(reused)

    def test_error_roster_order_is_part_of_the_contract(self) -> None:
        # Six languages index this list, so a reordering is a breaking change a
        # set comparison would silently accept.
        document = copy.deepcopy(self.manifest)
        codes = list(document["stable_error_codes"])
        codes[0], codes[1] = codes[1], codes[0]
        document["stable_error_codes"] = codes
        with self.assertRaisesRegex(
            conformance.D18TConformanceError, "exactly the D18T roster, in order"
        ):
            conformance.validate_manifest(document)

    def test_missing_or_extra_top_level_keys_are_rejected(self) -> None:
        without = copy.deepcopy(self.manifest)
        del without["race_traces"]
        with self.assertRaisesRegex(conformance.D18TConformanceError, "key roster"):
            conformance.validate_manifest(without)

        extra = copy.deepcopy(self.manifest)
        extra["unexpected"] = "value"
        with self.assertRaisesRegex(conformance.D18TConformanceError, "key roster"):
            conformance.validate_manifest(extra)

    def test_duplicate_json_keys_are_rejected(self) -> None:
        # Python's json would keep the last value, so the six ports could end up
        # reading different rosters from the same bytes.
        with self.assertRaisesRegex(
            conformance.D18TConformanceError, "duplicate JSON key"
        ):
            conformance._reject_duplicate_pairs([("a", 1), ("a", 2)])

    def test_constant_drift_is_rejected(self) -> None:
        document = copy.deepcopy(self.manifest)
        document["constants"]["max_cas_attempts"] = "32"
        with self.assertRaisesRegex(
            conformance.D18TConformanceError, "constant max_cas_attempts"
        ):
            conformance.validate_manifest(document)

    def test_a_changed_trace_expectation_is_rejected(self) -> None:
        # The traces ARE the crash and concurrency contract. Accepting "some
        # known outcome" would let one quietly change what it asserts.
        document = copy.deepcopy(self.manifest)
        for trace in document["crash_replay_traces"]:
            if trace["name"] == "after-all-grants":
                trace["expected"] = "prepared"
        with self.assertRaisesRegex(conformance.D18TConformanceError, "after-all-grants"):
            conformance.validate_manifest(document)

    def test_a_reordered_trace_roster_is_rejected(self) -> None:
        document = copy.deepcopy(self.manifest)
        document["race_traces"] = list(reversed(document["race_traces"]))
        with self.assertRaisesRegex(conformance.D18TConformanceError, "roster drifted"):
            conformance.validate_manifest(document)

    def test_a_repeated_test_only_secret_is_rejected(self) -> None:
        # A second occurrence means a key leaked into a summary, a public
        # record, or an expected-error string.
        document = copy.deepcopy(self.manifest)
        secret = document["test_only_secrets"]["next_cmk_hex"]
        document["activation_case"]["name"] = secret
        with self.assertRaisesRegex(
            conformance.D18TConformanceError, "appears more than once"
        ):
            conformance.validate_manifest(document)

    def test_prospective_revocation_cannot_silently_invert(self) -> None:
        document = copy.deepcopy(self.manifest)
        document["activation_case"]["receiver_a_retains_epochs"] = ["0", "1"]
        with self.assertRaisesRegex(
            conformance.D18TConformanceError, "revoked receiver must retain only"
        ):
            conformance.validate_manifest(document)

        granted = copy.deepcopy(self.manifest)
        granted["activation_case"]["receiver_a_new_grant"] = "anything"
        with self.assertRaisesRegex(
            conformance.D18TConformanceError, "must receive no new grant"
        ):
            conformance.validate_manifest(granted)

    def test_a_non_successor_activation_case_is_rejected(self) -> None:
        document = copy.deepcopy(self.manifest)
        document["activation_case"]["new_epoch"] = "2"
        with self.assertRaisesRegex(
            conformance.D18TConformanceError, "exactly one epoch"
        ):
            conformance.validate_manifest(document)

    def test_a_mismatched_plan_record_key_is_rejected(self) -> None:
        document = copy.deepcopy(self.manifest)
        document["activation_case"]["plan_record_key"] = "wrong/key"
        with self.assertRaisesRegex(
            conformance.D18TConformanceError, "plan record key"
        ):
            conformance.validate_manifest(document)

    def test_migration_vectors_must_really_be_v1_and_v2(self) -> None:
        document = copy.deepcopy(self.manifest)
        first = document["state_migrations"][0]
        first["d18s_v1_b64"], first["d18s_v2_b64"] = (
            first["d18s_v2_b64"],
            first["d18s_v1_b64"],
        )
        with self.assertRaisesRegex(
            conformance.D18TConformanceError, "not a D18S version 1 record"
        ):
            conformance.validate_manifest(document)

    def test_a_malformed_generator_hash_is_rejected(self) -> None:
        document = copy.deepcopy(self.manifest)
        document["generator_blob_sha1"] = "short"
        with self.assertRaisesRegex(
            conformance.D18TConformanceError, "git blob SHA-1"
        ):
            conformance.validate_manifest(document)

    def test_the_reference_must_claim_guaranteed_erasure(self) -> None:
        # Only the Rust reference claims this; the ports report their own honest
        # capability. If the manifest ever softened, the ports' assertions that
        # they differ would become vacuous.
        document = copy.deepcopy(self.manifest)
        document["secret_erasure_capability"] = "best_effort"
        with self.assertRaisesRegex(
            conformance.D18TConformanceError, "guaranteed secret erasure"
        ):
            conformance.validate_manifest(document)

    def test_a_manifest_that_does_not_name_its_spec_is_rejected(self) -> None:
        document = copy.deepcopy(self.manifest)
        document["spec"] = "code/specs/something-else.md"
        with self.assertRaisesRegex(
            conformance.D18TConformanceError, "does not point at its own spec"
        ):
            conformance.validate_manifest(document)

    def test_git_blob_sha1_matches_git(self) -> None:
        # "blob <len>\0" + content, per git's object format.
        self.assertEqual(
            "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
            conformance.git_blob_sha1(b""),
        )

    def test_strict_base64_rejects_sloppy_encodings(self) -> None:
        with self.assertRaisesRegex(conformance.D18TConformanceError, "strict base64"):
            conformance._strict_base64("not base64!", "context")

    def test_every_consumer_carries_the_required_markers(self) -> None:
        for lane in conformance.LANES:
            consumer = (ROOT / lane.consumer_test).read_text(encoding="utf-8")
            for marker in conformance.CONSUMER_MARKERS:
                self.assertIn(
                    marker,
                    consumer,
                    f"{lane.lane_id} consumer must assert {marker}",
                )

    def test_every_lane_package_has_a_build_front_door(self) -> None:
        for lane in conformance.LANES:
            build = ROOT / lane.package_root / "BUILD"
            self.assertTrue(build.is_file(), f"{lane.lane_id} must have a BUILD")
            commands = [
                line.strip()
                for line in build.read_text(encoding="utf-8").splitlines()
                if line.strip() and not line.lstrip().startswith("#")
            ]
            self.assertTrue(commands, f"{lane.lane_id} BUILD must carry commands")

    def test_the_checked_in_manifest_is_valid(self) -> None:
        conformance.validate_manifest(self.manifest)

    def test_load_manifest_rejects_oversize_and_non_object_documents(self) -> None:
        # Replaces a tautological "strict parse equals lenient parse" check --
        # load_manifest has already rejected duplicates and non-finite numbers
        # by then, so the two could never differ.
        with tempfile.TemporaryDirectory() as directory:
            oversize = Path(directory) / "big.json"
            oversize.write_bytes(b" " * (conformance.MAXIMUM_MANIFEST_BYTES + 1))
            with self.assertRaisesRegex(
                conformance.D18TConformanceError, "exceeds the safety limit"
            ):
                conformance.load_manifest(oversize)

            array = Path(directory) / "array.json"
            array.write_text("[]", encoding="utf-8")
            with self.assertRaisesRegex(
                conformance.D18TConformanceError, "must be a JSON object"
            ):
                conformance.load_manifest(array)

            broken = Path(directory) / "broken.json"
            broken.write_text("{", encoding="utf-8")
            with self.assertRaisesRegex(
                conformance.D18TConformanceError, "not valid JSON"
            ):
                conformance.load_manifest(broken)

            missing = Path(directory) / "absent.json"
            with self.assertRaisesRegex(conformance.D18TConformanceError, "cannot read"):
                conformance.load_manifest(missing)

    def test_non_finite_numbers_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "nan.json"
            path.write_text('{"value": NaN}', encoding="utf-8")
            with self.assertRaisesRegex(
                conformance.D18TConformanceError, "not valid JSON|non-finite"
            ):
                conformance.load_manifest(path)


if __name__ == "__main__":
    unittest.main()
