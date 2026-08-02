#!/usr/bin/env python3
"""Tests for the NN10 recurrent BPTT fixture validator."""

from __future__ import annotations

import importlib.util
import json
import unittest
from copy import deepcopy
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
MODULE_PATH = REPO_ROOT / "code" / "scripts" / "validate_recurrent_bptt_labs.py"
SPEC = importlib.util.spec_from_file_location("validate_recurrent_bptt_labs", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)


class RecurrentBpttLabTests(unittest.TestCase):
    def setUp(self) -> None:
        fixture_path = (
            REPO_ROOT
            / "code"
            / "specs"
            / "fixtures"
            / "recurrent-bptt-v1"
            / "labs"
            / "00-final-state-loss.json"
        )
        self.lab = json.loads(fixture_path.read_text(encoding="utf-8"))

    def test_validates_checked_in_corpus(self) -> None:
        paths = validator.validate_corpus()
        self.assertEqual([path.name for path in paths], ["00-final-state-loss.json"])

    def test_backpropagates_state_gradient_in_reverse_time(self) -> None:
        trace = validator.execute_lab(self.lab)
        self.assertEqual(
            [step["time"] for step in trace["backward_steps"]], [2, 1, 0]
        )
        self.assertEqual(
            [step["state_gradient"] for step in trace["backward_steps"]],
            [0.75, 0.375, 0.1875],
        )

    def test_accumulates_each_shared_parameter_gradient(self) -> None:
        trace = validator.execute_lab(self.lab)
        self.assertEqual(
            trace["gradient_totals"],
            {
                "input_weight": 0.9375,
                "recurrent_weight": 3.0,
                "bias": 1.3125,
                "initial_state": 0.09375,
            },
        )

    def test_matches_central_finite_differences(self) -> None:
        trace = validator.execute_lab(self.lab)
        self.assertLess(trace["gradient_check"]["max_absolute_error"], 1e-9)

    def test_proposes_an_update_that_reduces_loss(self) -> None:
        trace = validator.execute_lab(self.lab)
        self.assertAlmostEqual(
            trace["update"]["parameters"]["recurrent_weight"], 0.2
        )
        self.assertLess(trace["update"]["loss"], trace["forward"]["loss"])

    def test_rejects_a_mutated_time_local_contribution(self) -> None:
        lab = deepcopy(self.lab)
        lab["expected"]["backward_steps"][0]["recurrent_weight_gradient"] = 0
        with self.assertRaises(validator.RecurrentBpttValidationError):
            validator.validate_lab(lab)


if __name__ == "__main__":
    unittest.main()
