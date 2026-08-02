#!/usr/bin/env python3
"""Tests for the NN12 attention QKV fixture validator."""

from __future__ import annotations

import importlib.util
import json
import unittest
from copy import deepcopy
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
MODULE_PATH = REPO_ROOT / "code" / "scripts" / "validate_attention_qkv_labs.py"
SPEC = importlib.util.spec_from_file_location(
    "validate_attention_qkv_labs", MODULE_PATH
)
assert SPEC is not None and SPEC.loader is not None
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)


class AttentionQkvLabTests(unittest.TestCase):
    def setUp(self) -> None:
        path = (
            REPO_ROOT
            / "code"
            / "specs"
            / "fixtures"
            / "attention-qkv-v1"
            / "labs"
            / "00-three-token-qkv.json"
        )
        self.lab = json.loads(path.read_text(encoding="utf-8"))

    def test_validates_checked_in_corpus(self) -> None:
        paths = validator.validate_corpus()
        self.assertEqual([path.name for path in paths], ["00-three-token-qkv.json"])

    def test_projects_three_distinct_roles(self) -> None:
        trace = validator.execute_lab(self.lab)
        self.assertEqual(trace["queries"], [[1, 0], [0, 1], [1, 1]])
        self.assertEqual(trace["keys"], [[1, 1], [-1, 1], [0, 2]])
        self.assertEqual(trace["values"], [[2, 0], [0, 1], [2, 1]])

    def test_exposes_every_dot_product(self) -> None:
        trace = validator.execute_lab(self.lab)
        self.assertEqual(len(trace["dot_products"]), 9)
        blue_purple = trace["dot_products"][5]
        self.assertEqual(blue_purple["products"], [0, 2])
        self.assertEqual(blue_purple["raw_score"], 2)

    def test_builds_query_by_key_score_matrix(self) -> None:
        trace = validator.execute_lab(self.lab)
        self.assertEqual(trace["raw_score_matrix"], [[1, -1, 0], [1, 1, 2], [2, 0, 2]])

    def test_scales_by_square_root_of_key_dimension(self) -> None:
        trace = validator.execute_lab(self.lab)
        self.assertAlmostEqual(trace["scaled_score_matrix"][1][2], 2**0.5)
        self.assertAlmostEqual(trace["scaled_score_matrix"][0][1], -(0.5**0.5))

    def test_rejects_a_mutated_score(self) -> None:
        lab = deepcopy(self.lab)
        lab["expected"]["dot_products"][5]["raw_score"] = 3
        with self.assertRaises(validator.AttentionQkvValidationError):
            validator.validate_lab(lab)


if __name__ == "__main__":
    unittest.main()
