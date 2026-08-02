#!/usr/bin/env python3
"""Tests for the NN07 tiny-image CNN fixture validator."""

from __future__ import annotations

import importlib.util
import json
import unittest
from copy import deepcopy
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
MODULE_PATH = REPO_ROOT / "code" / "scripts" / "validate_tiny_image_cnn_labs.py"
SPEC = importlib.util.spec_from_file_location("validate_tiny_image_cnn_labs", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)


class TinyImageCnnLabTests(unittest.TestCase):
    def setUp(self) -> None:
        fixture_path = (
            REPO_ROOT
            / "code"
            / "specs"
            / "fixtures"
            / "tiny-image-cnn-v1"
            / "labs"
            / "00-two-channel-image.json"
        )
        self.lab = json.loads(fixture_path.read_text(encoding="utf-8"))

    def test_validates_checked_in_corpus(self) -> None:
        paths = validator.validate_corpus()
        self.assertEqual([path.name for path in paths], ["00-two-channel-image.json"])

    def test_accumulates_input_channels_and_bias(self) -> None:
        trace = validator.trace_pipeline(deepcopy(self.lab))
        self.assertEqual(
            trace["channel_contributions"][0],
            [[[0, 0], [4, 4]], [[0, 2], [0, 2]]],
        )
        self.assertEqual(trace["convolution"][0], [[0, 2], [4, 6]])
        self.assertEqual(trace["convolution"][1], [[6, 4], [2, 0]])

    def test_normalizes_each_output_channel_over_space(self) -> None:
        trace = validator.trace_pipeline(deepcopy(self.lab))
        normalization = trace["normalization"]
        self.assertEqual(normalization["means"], [3, 3])
        self.assertEqual(normalization["variances"], [5, 5])
        self.assertEqual(normalization["denominators"], [3, 3])
        self.assertAlmostEqual(normalization["maps"][0][0][1], -1 / 3)

    def test_relu_and_pooling_preserve_winner_positions(self) -> None:
        trace = validator.trace_pipeline(deepcopy(self.lab))
        self.assertEqual(trace["activation"][0], [[0, 0], [1 / 3, 1]])
        self.assertEqual(trace["activation"][1], [[1, 1 / 3], [0, 0]])
        self.assertEqual(trace["pooling"], {"values": [1, 1], "argmax": [[1, 1], [0, 0]]})

    def test_rejects_a_mutated_channel_contribution(self) -> None:
        lab = deepcopy(self.lab)
        lab["expected"]["channel_contributions"][0][1][0][1] += 0.5
        with self.assertRaises(validator.TinyImageCnnValidationError):
            validator.validate_lab(lab)

    def test_rejects_a_filter_without_every_input_channel(self) -> None:
        lab = deepcopy(self.lab)
        lab["filters"][0]["kernels"].pop()
        with self.assertRaises(validator.TinyImageCnnValidationError):
            validator.validate_lab(lab)


if __name__ == "__main__":
    unittest.main()
