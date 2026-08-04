#!/usr/bin/env python3
"""Tests for the NN09 recurrent-unroll fixture validator."""

from __future__ import annotations

import importlib.util
import json
import unittest
from copy import deepcopy
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
MODULE_PATH = REPO_ROOT / "code" / "scripts" / "validate_recurrent_unroll_labs.py"
SPEC = importlib.util.spec_from_file_location("validate_recurrent_unroll_labs", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)


class RecurrentUnrollLabTests(unittest.TestCase):
    def setUp(self) -> None:
        fixture_path = (
            REPO_ROOT
            / "code"
            / "specs"
            / "fixtures"
            / "recurrent-unroll-v1"
            / "labs"
            / "00-three-step-relu-state.json"
        )
        self.lab = json.loads(fixture_path.read_text(encoding="utf-8"))

    def test_validates_checked_in_corpus(self) -> None:
        paths = validator.validate_corpus()
        self.assertEqual(
            [path.name for path in paths], ["00-three-step-relu-state.json"]
        )

    def test_unrolls_three_steps_with_one_shared_parameter_set(self) -> None:
        trace = validator.trace_recurrent(
            self.lab["inputs"], self.lab["initial_state"], self.lab["parameters"]
        )
        self.assertEqual(trace["states"], [1, 3.5, 0.75])
        self.assertEqual(trace["final_state"], 0.75)

    def test_exposes_every_term_at_the_zero_input_step(self) -> None:
        trace = validator.trace_recurrent(
            self.lab["inputs"], self.lab["initial_state"], self.lab["parameters"]
        )
        final = trace["steps"][2]
        self.assertEqual(final["input_product"], 0)
        self.assertEqual(final["recurrent_product"], 1.75)
        self.assertEqual(final["preactivation"], 0.75)
        self.assertEqual(final["state"], 0.75)

    def test_memory_ablation_removes_the_final_state(self) -> None:
        trace = validator.trace_recurrent(
            self.lab["inputs"],
            self.lab["initial_state"],
            self.lab["parameters"],
            recurrent_enabled=False,
        )
        self.assertEqual(trace["states"], [1, 3, 0])

    def test_rejects_a_mutated_recurrent_product(self) -> None:
        lab = deepcopy(self.lab)
        lab["expected"]["steps"][1]["recurrent_product"] = 0
        with self.assertRaises(validator.RecurrentUnrollValidationError):
            validator.validate_lab(lab)

    def test_rejects_a_fourth_time_step(self) -> None:
        lab = deepcopy(self.lab)
        lab["inputs"].append(1)
        with self.assertRaises(validator.RecurrentUnrollValidationError):
            validator.validate_lab(lab)


if __name__ == "__main__":
    unittest.main()
