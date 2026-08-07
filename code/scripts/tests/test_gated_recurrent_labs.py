#!/usr/bin/env python3
"""Tests for the NN11 gated recurrent fixture validator."""

from __future__ import annotations

import importlib.util
import json
import unittest
from copy import deepcopy
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
MODULE_PATH = REPO_ROOT / "code" / "scripts" / "validate_gated_recurrent_labs.py"
SPEC = importlib.util.spec_from_file_location(
    "validate_gated_recurrent_labs", MODULE_PATH
)
assert SPEC is not None and SPEC.loader is not None
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)


class GatedRecurrentLabTests(unittest.TestCase):
    def setUp(self) -> None:
        fixture_path = (
            REPO_ROOT
            / "code"
            / "specs"
            / "fixtures"
            / "gated-recurrent-v1"
            / "labs"
            / "00-gru-lstm-gates.json"
        )
        self.lab = json.loads(fixture_path.read_text(encoding="utf-8"))

    def test_validates_checked_in_corpus(self) -> None:
        paths = validator.validate_corpus()
        self.assertEqual([path.name for path in paths], ["00-gru-lstm-gates.json"])

    def test_reproduces_every_gate_activation(self) -> None:
        trace = validator.execute_lab(self.lab)
        self.assertEqual(trace["gru"]["reset_gate"]["value"], 0.5)
        self.assertEqual(trace["gru"]["update_gate"]["value"], 0.25)
        self.assertEqual(trace["lstm"]["forget_gate"]["value"], 0.5)
        self.assertEqual(trace["lstm"]["input_gate"]["value"], 0.25)
        self.assertEqual(trace["lstm"]["output_gate"]["value"], 0.75)

    def test_compares_one_state_with_a_private_cell(self) -> None:
        trace = validator.execute_lab(self.lab)
        self.assertAlmostEqual(trace["gru"]["output"]["hidden_state"], 0.75)
        self.assertAlmostEqual(trace["lstm"]["output"]["cell_state"], 0.55)
        self.assertAlmostEqual(
            trace["lstm"]["output"]["hidden_state"], 0.3753901583926764
        )

    def test_reset_gate_changes_candidate_construction(self) -> None:
        trace = validator.execute_lab(self.lab)
        reset_off = trace["counterfactuals"]["gru"][2]
        self.assertEqual(reset_off["gate"], "reset")
        self.assertAlmostEqual(reset_off["candidate"], 0.2850288981936261)
        self.assertLess(reset_off["hidden_state"], 0.75)

    def test_output_gate_hides_but_does_not_erase_lstm_cell(self) -> None:
        trace = validator.execute_lab(self.lab)
        output_off = trace["counterfactuals"]["lstm"][2]
        output_on = trace["counterfactuals"]["lstm"][3]
        self.assertEqual(output_off["cell_state"], output_on["cell_state"])
        self.assertEqual(output_off["hidden_state"], 0)
        self.assertGreater(output_on["hidden_state"], 0.5)

    def test_rejects_a_mutated_gate_trace(self) -> None:
        lab = deepcopy(self.lab)
        lab["lstm"]["output"]["cell_state"] = 0.7
        with self.assertRaises(validator.GatedRecurrentValidationError):
            validator.validate_lab(lab)


if __name__ == "__main__":
    unittest.main()
