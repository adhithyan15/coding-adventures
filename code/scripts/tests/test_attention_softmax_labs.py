#!/usr/bin/env python3
"""Tests for the NN13 attention softmax fixture validator."""

from __future__ import annotations

import importlib.util
import json
import math
import unittest
from copy import deepcopy
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
MODULE_PATH = REPO_ROOT / "code" / "scripts" / "validate_attention_softmax_labs.py"
SPEC = importlib.util.spec_from_file_location(
    "validate_attention_softmax_labs", MODULE_PATH
)
assert SPEC is not None and SPEC.loader is not None
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)


class AttentionSoftmaxLabTests(unittest.TestCase):
    def setUp(self) -> None:
        path = (
            REPO_ROOT
            / "code"
            / "specs"
            / "fixtures"
            / "attention-softmax-v1"
            / "labs"
            / "00-three-token-causal-softmax.json"
        )
        self.lab = json.loads(path.read_text(encoding="utf-8"))

    def test_validates_checked_in_corpus(self) -> None:
        paths = validator.validate_corpus()
        self.assertEqual(
            [path.name for path in paths], ["00-three-token-causal-softmax.json"]
        )

    def test_every_softmax_row_sums_to_one(self) -> None:
        trace = validator.execute_lab(self.lab)
        for mode in ("unmasked", "causal"):
            for row in trace[mode]:
                self.assertAlmostEqual(sum(row["weights"]), 1)

    def test_causal_mask_zeros_every_future_weight(self) -> None:
        rows = validator.execute_lab(self.lab)["causal"]
        for query_index, row in enumerate(rows):
            for key_index in range(query_index + 1, 3):
                self.assertIsNone(row["masked_scores"][key_index])
                self.assertEqual(row["exponentials"][key_index], 0)
                self.assertEqual(row["weights"][key_index], 0)

    def test_blue_causal_row_is_hand_calculable(self) -> None:
        row = validator.execute_lab(self.lab)["causal"][1]
        self.assertEqual(row["shifted_scores"], [0, 0, None])
        self.assertEqual(row["exponentials"], [1, 1, 0])
        self.assertEqual(row["denominator"], 2)
        self.assertEqual(row["weights"], [0.5, 0.5, 0])
        self.assertEqual(row["context"], [1, 0.5])

    def test_unmasked_blue_can_read_the_future_value(self) -> None:
        row = validator.execute_lab(self.lab)["unmasked"][1]
        self.assertAlmostEqual(row["weights"][2], 0.5034898434845538)
        self.assertAlmostEqual(row["context"][0], 1.5034898434845538)
        self.assertAlmostEqual(row["context"][1], 0.7517449217422769)

    def test_max_subtraction_keeps_large_scores_finite(self) -> None:
        lab = deepcopy(self.lab)
        lab["scaled_score_matrix"][0] = [1000, 999, 998]
        row = validator.execute_lab(lab)["unmasked"][0]
        self.assertEqual(row["shifted_scores"], [0, -1, -2])
        self.assertTrue(all(math.isfinite(weight) for weight in row["weights"]))
        self.assertAlmostEqual(sum(row["weights"]), 1)

    def test_rejects_a_mutated_attention_weight(self) -> None:
        lab = deepcopy(self.lab)
        lab["expected"]["causal"][1]["weights"][0] = 0.6
        with self.assertRaises(validator.AttentionSoftmaxValidationError):
            validator.validate_lab(lab)


if __name__ == "__main__":
    unittest.main()
