#!/usr/bin/env python3
"""Tests for the NN04 optimization-learning fixture validator."""

from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from copy import deepcopy
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
MODULE_PATH = REPO_ROOT / "code" / "scripts" / "validate_optimization_learning_labs.py"
SPEC = importlib.util.spec_from_file_location(
    "validate_optimization_learning_labs", MODULE_PATH
)
assert SPEC is not None and SPEC.loader is not None
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)


class OptimizationLearningLabTests(unittest.TestCase):
    def setUp(self) -> None:
        fixture_path = (
            REPO_ROOT
            / "code"
            / "specs"
            / "fixtures"
            / "optimization-learning-v1"
            / "labs"
            / "00-linear-loss-landscape.json"
        )
        self.lab = json.loads(fixture_path.read_text(encoding="utf-8"))

    def test_validates_checked_in_corpus(self) -> None:
        paths = validator.validate_corpus()
        self.assertEqual(
            [path.name for path in paths], ["00-linear-loss-landscape.json"]
        )

    def test_reproduces_gradient_and_optimizer_oracles(self) -> None:
        rows = validator._rows(self.lab)
        parameters = self.lab["model"]["parameters"]
        self.assertEqual(
            validator.analytical_gradient(rows, parameters),
            {"weight": -8.5, "bias": -4.5},
        )
        full_batch = validator.run_strategy(self.lab, "full-batch")
        self.assertAlmostEqual(full_batch["loss"], 0.13251310567040367)

    def test_rejects_a_mutated_numerical_gradient(self) -> None:
        lab = deepcopy(self.lab)
        lab["gradient_check"]["expected"]["numerical"]["weight"] += 0.1
        with self.assertRaises(validator.OptimizationLabValidationError):
            validator.validate_lab(lab)

    def test_rejects_a_missing_batch_strategy(self) -> None:
        lab = deepcopy(self.lab)
        lab["optimizer_comparison"]["strategies"].pop()
        with self.assertRaises(validator.OptimizationLabValidationError):
            validator.validate_lab(lab)

    def test_cli_rejects_an_empty_corpus(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            schema = {"$schema": "https://json-schema.org/draft/2020-12/schema"}
            (root / "schema.json").write_text(json.dumps(schema), encoding="utf-8")
            (root / "labs").mkdir()
            with self.assertRaises(validator.OptimizationLabValidationError):
                validator.validate_corpus(root)


if __name__ == "__main__":
    unittest.main()
