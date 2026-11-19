"""Falsification tests for the DT18 hash-map fixture oracle and streams."""

from __future__ import annotations

import copy
import unittest

from generate import make_cases
from hash_map_oracle import (
    HISTORY_COUNT,
    decode_history_ordinal,
    history_ordinal,
    physical_order_table,
)
from validate import (
    FixtureValidationError,
    validate_bytes,
    validate_document,
    validate_file,
)


class HashMapFixtureValidationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.document = make_cases()

    def test_canonical_fixture_is_byte_stable_and_model_valid(self) -> None:
        document, digest = validate_file()
        self.assertEqual(document, self.document)
        self.assertEqual(len(digest), 64)

    def test_all_history_ordinals_biject_with_length_lexicographic_domain(self) -> None:
        decoded = [decode_history_ordinal(index) for index in range(HISTORY_COUNT)]
        self.assertEqual(len(decoded), HISTORY_COUNT)
        self.assertEqual(len(set(decoded)), HISTORY_COUNT)
        self.assertTrue(
            all(
                history_ordinal(history) == index
                for index, history in enumerate(decoded)
            )
        )
        self.assertEqual(decoded[0], ())
        self.assertEqual(decoded[1], (0,))
        self.assertEqual(decoded[9], (8,))
        self.assertEqual(decoded[10], (0, 0))
        self.assertEqual(decoded[-1], (8, 8, 8, 8, 8))

    def test_physical_order_table_contains_every_ordered_subset_once(self) -> None:
        table = physical_order_table()
        self.assertEqual(len(table), 16)
        self.assertEqual(len({tuple(row) for row in table}), 16)
        self.assertEqual(table[0], [])
        self.assertEqual(table[1], ["k0"])
        self.assertEqual(table[-1], ["k2", "k1", "k0"])

    def test_literal_key_framing_and_known_hash_answers(self) -> None:
        keys = {row["id"]: row for row in self.document["key_cases"]}
        self.assertEqual(keys["bytes-empty"]["encoded_hex"], "000000000000000000")
        self.assertEqual(keys["string-empty"]["encoded_hex"], "010000000000000000")
        self.assertEqual(keys["k0"]["encoded_hex"], "00010000000000000000")
        self.assertEqual(keys["string-a"]["encoded_hex"], "01010000000000000061")
        answers = {
            row["id"]: row["expected_hex"] for row in self.document["hash_answers"]
        }
        self.assertEqual(answers["fnv32-abc-known-answer"], "1a47e90b")
        self.assertEqual(answers["murmur-seeded-fox-known-answer"], "2fa826cd")
        self.assertEqual(answers["siphash-caller-key-known-answer"], "3f2acc7f57c29bdb")

    def test_truncated_stream_is_rejected_before_model_comparison(self) -> None:
        document = copy.deepcopy(self.document)
        stream = document["history_recipe"]["expected_semantic_streams"][
            "linearProbing"
        ]
        document["history_recipe"]["expected_semantic_streams"]["linearProbing"] = (
            stream[:-1]
        )
        with self.assertRaisesRegex(
            FixtureValidationError,
            "schema history_recipe.expected_semantic_streams.linearProbing",
        ):
            validate_document(document)

    def test_malformed_stream_alphabet_is_rejected(self) -> None:
        document = copy.deepcopy(self.document)
        stream = document["history_recipe"]["expected_physical_order_streams"][
            "separateChaining"
        ]
        document["history_recipe"]["expected_physical_order_streams"][
            "separateChaining"
        ] = "g" + stream[1:]
        with self.assertRaises(FixtureValidationError):
            validate_document(document)

    def test_reordered_history_operation_alphabet_is_rejected(self) -> None:
        document = copy.deepcopy(self.document)
        (
            document["history_recipe"]["operations"][0],
            document["history_recipe"]["operations"][1],
        ) = (
            document["history_recipe"]["operations"][1],
            document["history_recipe"]["operations"][0],
        )
        with self.assertRaisesRegex(
            FixtureValidationError, "operation alphabet or order changed"
        ):
            validate_document(document)

    def test_well_formed_one_output_substitution_is_rejected_by_model(self) -> None:
        document = copy.deepcopy(self.document)
        stream = document["history_recipe"]["expected_semantic_streams"][
            "separateChaining"
        ]
        replacement = "1" if stream[2] != "1" else "2"
        document["history_recipe"]["expected_semantic_streams"]["separateChaining"] = (
            stream[:2] + replacement + stream[3:]
        )
        with self.assertRaisesRegex(
            FixtureValidationError, "independent model mismatch"
        ):
            validate_document(document)

    def test_well_formed_stream_permutation_is_rejected(self) -> None:
        document = copy.deepcopy(self.document)
        stream = document["history_recipe"]["expected_physical_order_streams"][
            "linearProbing"
        ]
        document["history_recipe"]["expected_physical_order_streams"][
            "linearProbing"
        ] = stream[:2] + stream[3] + stream[2] + stream[4:]
        self.assertEqual(
            len(
                document["history_recipe"]["expected_physical_order_streams"][
                    "linearProbing"
                ]
            ),
            HISTORY_COUNT,
        )
        with self.assertRaisesRegex(
            FixtureValidationError, "independent model mismatch"
        ):
            validate_document(document)

    def test_well_formed_wrong_hash_control_targets_public_comparator(self) -> None:
        answer = next(
            row
            for row in self.document["hash_answers"]
            if row["id"] == "fnv32-abc-known-answer"
        )
        control = self.document["negative_controls"][0]
        mutated = f"{int(answer['expected_hex'], 16) ^ int(control['expected_xor_mask'], 16):08x}"
        self.assertEqual(mutated, control["mutated_expected_hex"])
        self.assertNotEqual(mutated, answer["expected_hex"])
        self.assertEqual(control["required_stage"], "actual-public-hash-comparator")
        self.assertEqual(len(mutated), 8)
        self.assertTrue(all(character in "0123456789abcdef" for character in mutated))

    def test_json_syntax_errors_are_reported_as_fixture_validation_errors(self) -> None:
        with self.assertRaisesRegex(FixtureValidationError, "not valid UTF-8 JSON"):
            validate_bytes(b'{"schema":')

    def test_full_cycle_witness_contains_fallback_and_threshold_growth(self) -> None:
        case = next(
            row
            for row in self.document["operation_cases"]
            if row["id"] == "linear-full-cycle-tombstone-fallback-and-growth"
        )
        self.assertEqual(case["steps"][7]["expected"]["state"]["count"], 0)
        self.assertEqual(case["steps"][7]["expected"]["state"]["entries"], [])
        self.assertEqual(
            case["steps"][8]["expected"]["state"]["entries"][0]["key_id"], "cycle4"
        )
        self.assertFalse(case["steps"][9]["expected"]["observation"]["has"])
        self.assertEqual(case["steps"][-2]["expected"]["state"]["count"], 3)
        self.assertEqual(case["steps"][-1]["expected"]["state"]["capacity"], 8)

    def test_null_value_case_also_pins_presence(self) -> None:
        case = next(
            row
            for row in self.document["operation_cases"]
            if row["id"] == "linear-wrap-tombstone-reuse"
        )
        observation = case["steps"][-1]["expected"]["observation"]
        self.assertEqual(observation, {"has": True, "value": None, "value_id": "null"})


if __name__ == "__main__":
    unittest.main()
