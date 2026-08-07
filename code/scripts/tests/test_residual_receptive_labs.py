#!/usr/bin/env python3
"""Tests for the NN08 residual/receptive-field fixture validator."""

from __future__ import annotations

import importlib.util
import json
import unittest
from copy import deepcopy
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
MODULE_PATH = REPO_ROOT / "code" / "scripts" / "validate_residual_receptive_labs.py"
SPEC = importlib.util.spec_from_file_location(
    "validate_residual_receptive_labs", MODULE_PATH
)
assert SPEC is not None and SPEC.loader is not None
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)


class ResidualReceptiveLabTests(unittest.TestCase):
    def setUp(self) -> None:
        fixture_path = (
            REPO_ROOT
            / "code"
            / "specs"
            / "fixtures"
            / "residual-receptive-v1"
            / "labs"
            / "00-two-layer-residual.json"
        )
        self.lab = json.loads(fixture_path.read_text(encoding="utf-8"))

    def test_validates_checked_in_corpus(self) -> None:
        paths = validator.validate_corpus()
        self.assertEqual([path.name for path in paths], ["00-two-layer-residual.json"])

    def test_runs_two_same_padded_layers_and_identity_skip(self) -> None:
        trace = validator.trace_residual(self.lab["input"], self.lab["kernels"])
        self.assertEqual(trace["hidden"], [1, 3, 2, 3, 1])
        self.assertEqual(trace["main"], [4, 6, 8, 6, 4])
        self.assertEqual(trace["residual_sum"], [5, 6, 10, 6, 5])
        self.assertEqual(trace["output"], [5, 6, 10, 6, 5])

    def test_expands_center_output_to_five_input_positions(self) -> None:
        trace = validator.trace_residual(self.lab["input"], self.lab["kernels"])
        center = trace["traces"][2]
        self.assertEqual(center["hidden_indices"], [1, 2, 3])
        self.assertEqual(center["input_path_counts"], [1, 2, 3, 2, 1])
        self.assertEqual(center["input_contributions"], [1, 0, 6, 0, 1])
        self.assertEqual(center["receptive_field_indices"], [0, 1, 2, 3, 4])

    def test_clips_in_range_receptive_fields_at_boundaries(self) -> None:
        trace = validator.trace_residual(self.lab["input"], self.lab["kernels"])
        self.assertEqual(trace["traces"][0]["receptive_field_indices"], [0, 1, 2])
        self.assertEqual(trace["traces"][4]["receptive_field_indices"], [2, 3, 4])

    def test_rejects_a_mutated_path_count(self) -> None:
        lab = deepcopy(self.lab)
        lab["expected"]["traces"][2]["input_path_counts"][2] = 2
        with self.assertRaises(validator.ResidualReceptiveValidationError):
            validator.validate_lab(lab)

    def test_rejects_a_non_identity_length_skip(self) -> None:
        lab = deepcopy(self.lab)
        lab["input"].pop()
        with self.assertRaises(validator.ResidualReceptiveValidationError):
            validator.validate_lab(lab)


if __name__ == "__main__":
    unittest.main()
