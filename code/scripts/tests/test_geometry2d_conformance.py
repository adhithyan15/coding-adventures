"""The neutral geometry oracle must reject corrupt or weakened cases."""

from __future__ import annotations

import copy
import importlib.util
import json
import pathlib
import unittest
from collections.abc import Callable
from typing import Any

from jsonschema import Draft202012Validator  # type: ignore[import-untyped]

ROOT = pathlib.Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "code" / "scripts" / "geometry2d_conformance.py"
CORPUS = ROOT / "code" / "specs" / "fixtures" / "geometry2d-v1" / "cases.json"
SCHEMA = CORPUS.with_name("schema.json")
SPEC = importlib.util.spec_from_file_location("geometry2d_conformance", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class Geometry2DConformanceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.document = json.loads(CORPUS.read_text(encoding="utf-8"))

    def test_corpus_and_closed_schema(self) -> None:
        schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
        Draft202012Validator.check_schema(schema)
        self.assertEqual(
            list(Draft202012Validator(schema).iter_errors(self.document)), []
        )
        self.assertEqual(MODULE.validate_document(self.document), 23)
        self.assertEqual(
            MODULE.validate_document(
                MODULE.parse_json(CORPUS.read_text(encoding="utf-8"))
            ),
            23,
        )

    def test_complete_operation_and_case_roster(self) -> None:
        expected = {
            "normalize-zero",
            "normalize-below-epsilon",
            "normalize-at-epsilon",
            "normalize-three-four",
            "arc-zero-radius-reversed",
            "arc-negative-near-zero-radius",
            "arc-near-coincident-endpoints",
            "arc-coincident-endpoints",
            "affine-compose-order-a",
            "affine-compose-order-b",
            "affine-invert-nonsingular",
            "affine-invert-singular",
            "affine-vector-translation",
            "bezier-quadratic-quarter",
            "bezier-cubic-quarter",
            "bezier-cubic-x-overshoot",
            "arc-center-bounds-positive-wrap",
            "arc-center-bounds-negative-wrap",
            "arc-center-bounds-zero-sweep",
            "arc-center-bounds-rotated-extrema",
            "arc-center-cubics-zero",
            "arc-center-cubics-full-turn",
            "arc-center-cubics-over-turn",
        }
        self.assertEqual({case["id"] for case in self.document["cases"]}, expected)

    def test_new_expected_values_cannot_drift(self) -> None:
        edits: list[tuple[str, Callable[[dict[str, Any]], None]]] = [
            (
                "affine-compose-order-a",
                lambda c: c["expected_matrix"].__setitem__(4, 8),
            ),
            (
                "affine-invert-nonsingular",
                lambda c: c["expected_inverse"].__setitem__(0, 0),
            ),
            ("affine-vector-translation", lambda c: c["expected"].__setitem__(0, 99)),
            (
                "bezier-quadratic-quarter",
                lambda c: c["expected_derivative"].__setitem__(1, 5),
            ),
            (
                "bezier-cubic-quarter",
                lambda c: c["expected_split"][0][1].__setitem__(1, 2),
            ),
            (
                "bezier-cubic-x-overshoot",
                lambda c: c["expected_bounds"].__setitem__(2, 4),
            ),
            (
                "arc-center-bounds-positive-wrap",
                lambda c: c["expected_bounds"].__setitem__(2, 0.58),
            ),
            (
                "arc-center-bounds-negative-wrap",
                lambda c: c["expected_bounds"].__setitem__(1, -0.5),
            ),
            (
                "arc-center-bounds-zero-sweep",
                lambda c: c["expected_bounds"].__setitem__(2, 1),
            ),
            (
                "arc-center-bounds-rotated-extrema",
                lambda c: c["expected_bounds"].__setitem__(2, 1),
            ),
            ("arc-center-cubics-zero", lambda c: c.update(expected_count=0)),
            ("arc-center-cubics-full-turn", lambda c: c.update(expected_count=5)),
            (
                "arc-center-cubics-over-turn",
                lambda c: c.update(expected_error="none"),
            ),
        ]
        for case_id, edit in edits:
            with self.subTest(case_id=case_id):
                changed = copy.deepcopy(self.document)
                case = next(item for item in changed["cases"] if item["id"] == case_id)
                edit(case)
                with self.assertRaisesRegex(ValueError, "expected"):
                    MODULE.validate_document(changed)

    def test_new_case_shapes_and_ranges_are_closed(self) -> None:
        for case_id, edit in [
            ("affine-compose-order-a", lambda c: c.update(first=[1, 0])),
            ("affine-invert-singular", lambda c: c.update(expected_inverse=[1] * 6)),
            ("bezier-quadratic-quarter", lambda c: c.update(control_points=[[0, 0]])),
            ("bezier-cubic-quarter", lambda c: c.update(t=2)),
            ("bezier-cubic-quarter", lambda c: c.update(unreviewed=True)),
            ("arc-center-bounds-positive-wrap", lambda c: c.update(rx=-1)),
            ("arc-center-cubics-zero", lambda c: c.update(sweep_angle=float("nan"))),
            ("arc-center-cubics-over-turn", lambda c: c.update(expected_count=5)),
        ]:
            with self.subTest(case_id=case_id):
                changed = copy.deepcopy(self.document)
                case = next(item for item in changed["cases"] if item["id"] == case_id)
                edit(case)
                with self.assertRaises(ValueError):
                    MODULE.validate_document(changed)

    def test_invalid_sweep_schema_branch_rejects_valid_sweep(self) -> None:
        schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
        changed = copy.deepcopy(self.document)
        case = next(
            item
            for item in changed["cases"]
            if item["id"] == "arc-center-cubics-over-turn"
        )
        case["sweep_angle"] = 0
        self.assertNotEqual(list(Draft202012Validator(schema).iter_errors(changed)), [])
        with self.assertRaises(ValueError):
            MODULE.validate_document(changed)

    def test_duplicate_keys_and_ids_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "duplicate JSON key"):
            MODULE.parse_json('{"version":1,"version":1}')
        changed = copy.deepcopy(self.document)
        changed["cases"][1]["id"] = changed["cases"][0]["id"]
        with self.assertRaisesRegex(ValueError, "duplicate case id"):
            MODULE.validate_document(changed)

    def test_extra_field_and_tolerance_drift_rejected(self) -> None:
        changed = copy.deepcopy(self.document)
        changed["cases"][0]["unreviewed"] = 1
        with self.assertRaisesRegex(ValueError, "fields"):
            MODULE.validate_document(changed)
        changed = copy.deepcopy(self.document)
        changed["absolute_tolerance"] = 1
        with self.assertRaisesRegex(ValueError, "tolerance"):
            MODULE.validate_document(changed)

    def test_mutated_expectations_rejected(self) -> None:
        changed = copy.deepcopy(self.document)
        changed["cases"][2]["expected"] = [0, 0]
        with self.assertRaisesRegex(ValueError, "expected"):
            MODULE.validate_document(changed)
        changed = copy.deepcopy(self.document)
        changed["cases"][1]["comparison"] = "absolute"
        with self.assertRaisesRegex(ValueError, "comparison"):
            MODULE.validate_document(changed)
        changed = copy.deepcopy(self.document)
        changed["cases"][1]["expected"] = [5e-16, 0]
        with self.assertRaisesRegex(ValueError, "exact origin"):
            MODULE.validate_document(changed)
        changed = copy.deepcopy(self.document)
        changed["cases"][4]["expected_bounds"][2] = 2.0000000000015
        with self.assertRaisesRegex(ValueError, "expected"):
            MODULE.validate_document(changed)
        changed = copy.deepcopy(self.document)
        changed["cases"][4]["expected_bounds"] = [3, 4, -2, -2]
        with self.assertRaisesRegex(ValueError, "expected"):
            MODULE.validate_document(changed)

    def test_nonfinite_and_nondegenerate_rejected(self) -> None:
        changed = copy.deepcopy(self.document)
        changed["cases"][0]["point"][0] = float("nan")
        with self.assertRaisesRegex(ValueError, "finite"):
            MODULE.validate_document(changed)
        changed = copy.deepcopy(self.document)
        changed["cases"][4]["rx"] = 1
        with self.assertRaisesRegex(ValueError, "not degenerate"):
            MODULE.validate_document(changed)
        changed = copy.deepcopy(self.document)
        changed["cases"][4]["rx"] = 1e-10
        with self.assertRaisesRegex(ValueError, "not degenerate"):
            MODULE.validate_document(changed)
        changed = copy.deepcopy(self.document)
        changed["cases"][6]["to"] = [1e-10, 0]
        with self.assertRaisesRegex(ValueError, "not degenerate"):
            MODULE.validate_document(changed)

    def test_shape_and_semantic_boundaries_rejected(self) -> None:
        edits: list[tuple[Callable[[dict[str, Any]], None], str]] = [
            (lambda d: d.update(version=2), "version"),
            (lambda d: d.update(cases=[]), "cases"),
            (lambda d: d["cases"][0].update(id="Bad_ID"), "case id"),
            (lambda d: d["cases"][0].update(operation="unknown"), "unsupported"),
            (lambda d: d["cases"][0].update(point=[0]), "exactly 2"),
            (lambda d: d["cases"][4].update(sample_t=2), "sample_t"),
            (lambda d: d["cases"][4].update(expected_point=[0, 0]), "expected"),
        ]
        for edit, error in edits:
            with self.subTest(error=error):
                changed = copy.deepcopy(self.document)
                edit(changed)
                with self.assertRaisesRegex(ValueError, error):
                    MODULE.validate_document(changed)

    def test_bad_case_type_and_root_extra_field_rejected(self) -> None:
        changed = copy.deepcopy(self.document)
        changed["cases"][0] = None
        with self.assertRaisesRegex(ValueError, "case fields"):
            MODULE.validate_document(changed)
        changed = copy.deepcopy(self.document)
        changed["extra"] = 1
        with self.assertRaisesRegex(ValueError, "fields"):
            MODULE.validate_document(changed)


if __name__ == "__main__":
    unittest.main()
