"""Negative controls for the same byte-pinned GAME frame validator."""

from __future__ import annotations

import hashlib
import json
import random
import unittest

from validate import CASE_IDS, FIXTURE, FixtureError, validate_bytes


def payload_with(mutator) -> bytes:
    document = json.loads(FIXTURE.read_bytes())
    mutator(document)
    return json.dumps(document, separators=(",", ":")).encode("utf-8")


def validate_repinned(raw: bytes) -> None:
    """Pin each mutated payload's bytes so validation reaches structure/model."""
    validate_bytes(raw, hashlib.sha256(raw).hexdigest(), announce=False)


class FixtureValidatorTests(unittest.TestCase):
    def test_canonical_bytes_have_exact_case_set(self) -> None:
        document = json.loads(FIXTURE.read_bytes())
        self.assertEqual({case["id"] for case in document["cases"]}, CASE_IDS)

    def test_duplicate_object_key_reaches_json_structure_validation(self) -> None:
        raw = FIXTURE.read_bytes().replace(
            b'"version": 1', b'"version": true, "version": 1', 1
        )
        self.assertNotEqual(hashlib.sha256(raw).hexdigest(), "8db2c0fdc9bb6ab93785f3bd6d5dc52183b67682ca0e8d693fcdbfaa30aff785")
        with self.assertRaisesRegex(FixtureError, "duplicate object key 'version'"):
            validate_repinned(raw)

    def test_boolean_version_is_not_integer_one(self) -> None:
        raw = payload_with(lambda data: data.__setitem__("version", True))
        with self.assertRaisesRegex(FixtureError, "unsupported fixture version"):
            validate_repinned(raw)

    def test_boolean_expected_player_is_rejected_before_equality(self) -> None:
        def mutate(data):
            case = next(row for row in data["cases"] if row["id"] == "press_from_up")
            case["expected"]["frame"]["player_id"] = True

        with self.assertRaisesRegex(FixtureError, "expected.frame.player_id"):
            validate_repinned(payload_with(mutate))

    def test_boolean_expected_axis_is_rejected_before_equality(self) -> None:
        def mutate(data):
            case = next(row for row in data["cases"] if row["id"] == "press_from_up")
            case["expected"]["frame"]["axes"][0] = False

        with self.assertRaisesRegex(FixtureError, r"expected.frame.axes\[0\]"):
            validate_repinned(payload_with(mutate))

    def test_boolean_expected_error_index_is_rejected_before_equality(self) -> None:
        def mutate(data):
            case = next(row for row in data["cases"] if row["id"] == "invalid_axis_value")
            case["expected"]["error"]["change_index"] = False

        with self.assertRaisesRegex(FixtureError, "expected.error.change_index"):
            validate_repinned(payload_with(mutate))

    def test_wrong_axis_domain_in_expected_frame_is_rejected(self) -> None:
        def mutate(data):
            case = next(row for row in data["cases"] if row["id"] == "press_from_up")
            case["expected"]["frame"]["axes"][0] = -32768

        with self.assertRaisesRegex(FixtureError, r"expected.frame.axes\[0\]"):
            validate_repinned(payload_with(mutate))

    def test_seeded_mutation_fuzz_reaches_the_repinned_json_boundary(self) -> None:
        seed = 0x4742_4C30_3546_5A5A
        rng = random.Random(seed)
        original = FIXTURE.read_bytes()
        rejected = 0
        accepted = 0
        for case_index in range(96):
            offset = rng.randrange(len(original))
            strategy = case_index % 4
            if strategy == 0:
                mutated = original[:offset] + original[offset + 1 :]
            elif strategy == 1:
                replacement = bytes([rng.randrange(32, 127)])
                mutated = original[:offset] + replacement + original[offset + 1 :]
            elif strategy == 2:
                mutated = original[:offset]
            else:
                token = (b'"' if case_index % 2 else b'}')
                mutated = original[:offset] + token + original[offset:]

            # Each payload is pinned independently so malformed-byte, duplicate,
            # schema, and semantic checks—not the canonical digest—decide it.
            digest = hashlib.sha256(mutated).hexdigest()
            try:
                validate_bytes(mutated, digest, announce=False)
            except (FixtureError, json.JSONDecodeError, UnicodeDecodeError):
                rejected += 1
            else:
                accepted += 1
        self.assertGreater(rejected, 0)
        self.assertEqual(accepted + rejected, 96)
        print(
            f"GBL050-FUZZ-JSON seed={seed:#014x} inputs=96 accepted={accepted} rejected={rejected}"
        )


if __name__ == "__main__":
    unittest.main()
