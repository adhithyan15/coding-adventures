#!/usr/bin/env python3
"""Tests for the NN05 convolution-learning fixture validator."""

from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from copy import deepcopy
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
MODULE_PATH = REPO_ROOT / "code" / "scripts" / "validate_convolution_learning_labs.py"
SPEC = importlib.util.spec_from_file_location(
    "validate_convolution_learning_labs", MODULE_PATH
)
assert SPEC is not None and SPEC.loader is not None
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)


class ConvolutionLearningLabTests(unittest.TestCase):
    def setUp(self) -> None:
        fixture_path = (
            REPO_ROOT
            / "code"
            / "specs"
            / "fixtures"
            / "convolution-learning-v1"
            / "labs"
            / "00-asymmetric-valid-kernel.json"
        )
        self.lab = json.loads(fixture_path.read_text(encoding="utf-8"))

    def test_validates_checked_in_corpus(self) -> None:
        paths = validator.validate_corpus()
        self.assertEqual(
            [path.name for path in paths], ["00-asymmetric-valid-kernel.json"]
        )

    def test_reproduces_every_multiply_accumulate(self) -> None:
        positions = validator.trace_valid_correlation(
            self.lab["signal"], self.lab["kernel"]
        )
        self.assertEqual([position["output"] for position in positions], [7, -2, 11, 0])
        self.assertEqual(positions[2]["products"], [3, 0, 8])
        self.assertEqual(positions[2]["accumulator"], [0.0, 3.0, 3.0, 11.0])

    def test_asymmetric_kernel_is_not_reversed(self) -> None:
        reversed_positions = validator.trace_valid_correlation(
            self.lab["signal"], list(reversed(self.lab["kernel"]))
        )
        self.assertEqual(
            [position["output"] for position in reversed_positions], [6, -1, 10, -2]
        )

    def test_rejects_a_mutated_product_trace(self) -> None:
        lab = deepcopy(self.lab)
        lab["expected"]["positions"][0]["products"][2] += 1
        with self.assertRaises(validator.ConvolutionLabValidationError):
            validator.validate_lab(lab)

    def test_rejects_an_invalid_output_count(self) -> None:
        lab = deepcopy(self.lab)
        lab["expected"]["outputs"].pop()
        with self.assertRaises(validator.ConvolutionLabValidationError):
            validator.validate_lab(lab)

    def test_cli_rejects_an_empty_corpus(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            schema = {"$schema": "https://json-schema.org/draft/2020-12/schema"}
            (root / "schema.json").write_text(json.dumps(schema), encoding="utf-8")
            (root / "labs").mkdir()
            with self.assertRaises(validator.ConvolutionLabValidationError):
                validator.validate_corpus(root)


if __name__ == "__main__":
    unittest.main()
