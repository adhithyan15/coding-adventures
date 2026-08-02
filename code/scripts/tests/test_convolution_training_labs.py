#!/usr/bin/env python3
"""Tests for the NN06 convolution-training fixture validator."""

from __future__ import annotations

import importlib.util
import json
import unittest
from copy import deepcopy
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
MODULE_PATH = REPO_ROOT / "code" / "scripts" / "validate_convolution_training_labs.py"
SPEC = importlib.util.spec_from_file_location(
    "validate_convolution_training_labs", MODULE_PATH
)
assert SPEC is not None and SPEC.loader is not None
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)


class ConvolutionTrainingLabTests(unittest.TestCase):
    def setUp(self) -> None:
        fixture_path = (
            REPO_ROOT
            / "code"
            / "specs"
            / "fixtures"
            / "convolution-training-v1"
            / "labs"
            / "00-shared-kernel-gradient.json"
        )
        self.lab = json.loads(fixture_path.read_text(encoding="utf-8"))

    def test_validates_checked_in_corpus(self) -> None:
        paths = validator.validate_corpus()
        self.assertEqual(
            [path.name for path in paths], ["00-shared-kernel-gradient.json"]
        )

    def test_accumulates_every_shared_weight_gradient(self) -> None:
        trace = validator.trace_training(
            self.lab["signal"], self.lab["kernel"], self.lab["targets"]
        )
        self.assertEqual(trace["outputs"], [7, -2, 11, 0])
        self.assertEqual(trace["output_gradients"], [0.5, 0, 0.5, 0])
        self.assertEqual(trace["kernel_gradient"], [2.5, 0.5, 3.5])
        self.assertEqual(
            trace["contributions"][2]["kernel_gradient"], [1.5, 0, 2]
        )

    def test_analytical_gradient_matches_independent_finite_difference(self) -> None:
        trace = validator.trace_training(
            self.lab["signal"], self.lab["kernel"], self.lab["targets"]
        )
        numerical = validator.numerical_kernel_gradient(
            self.lab["signal"],
            self.lab["kernel"],
            self.lab["targets"],
            self.lab["gradient_check"]["epsilon"],
        )
        for analytical, estimate in zip(trace["kernel_gradient"], numerical):
            self.assertAlmostEqual(analytical, estimate, places=8)

    def test_one_step_reduces_loss(self) -> None:
        before = validator.trace_training(
            self.lab["signal"], self.lab["kernel"], self.lab["targets"]
        )
        after = validator.optimizer_step(
            self.lab["signal"],
            self.lab["kernel"],
            self.lab["targets"],
            self.lab["optimizer_step"]["learning_rate"],
        )
        self.assertEqual(after["kernel"], [0.95, -1.01, 1.93])
        self.assertAlmostEqual(after["loss"], 0.206525)
        self.assertLess(after["loss"], before["loss"])

    def test_rejects_a_mutated_contribution(self) -> None:
        lab = deepcopy(self.lab)
        lab["gradient_check"]["expected"]["contributions"][0][
            "kernel_gradient"
        ][1] += 0.25
        with self.assertRaises(validator.ConvolutionTrainingValidationError):
            validator.validate_lab(lab)

    def test_rejects_the_wrong_target_count(self) -> None:
        lab = deepcopy(self.lab)
        lab["targets"].pop()
        with self.assertRaises(validator.ConvolutionTrainingValidationError):
            validator.validate_lab(lab)


if __name__ == "__main__":
    unittest.main()
