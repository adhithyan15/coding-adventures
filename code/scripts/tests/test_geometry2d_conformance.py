"""The neutral geometry oracle must reject corrupt or weakened cases."""

from __future__ import annotations

import copy
import importlib.util
import json
import math
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
        self.assertEqual(MODULE.validate_document(self.document), 27)
        self.assertEqual(
            MODULE.validate_document(
                MODULE.parse_json(CORPUS.read_text(encoding="utf-8"))
            ),
            27,
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
            "arc-center-tangent-positive-axis",
            "arc-center-tangent-positive-rotated",
            "arc-center-tangent-negative-rotated",
            "arc-center-tangent-zero-rotated",
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

    def tangent_case(self, index: int = 0) -> dict[str, Any]:
        return copy.deepcopy(
            [c for c in self.document["cases"] if c["operation"] == "center-arc-tangent"][index]
        )

    def check_case(self, case: dict[str, Any]) -> None:
        document = {"version": 1, "absolute_tolerance": 1e-12, "cases": [case]}
        schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
        self.assertEqual(list(Draft202012Validator(schema).iter_errors(document)), [])
        self.assertEqual(MODULE.validate_document(document), 1)

    def reject_case(self, case: dict[str, Any], *, schema: bool = True) -> None:
        document = {"version": 1, "absolute_tolerance": 1e-12, "cases": [case]}
        if schema:
            validator = Draft202012Validator(json.loads(SCHEMA.read_text(encoding="utf-8")))
            self.assertNotEqual(list(validator.iter_errors(document)), [])
        with self.assertRaises(ValueError):
            MODULE.validate_document(document)

    def test_tangent_constants_translation_and_ellipse_gradient(self) -> None:
        # These pi expressions are independent of the oracle's coordinate formula.
        answers = [(0, math.pi / 2), (-3 * math.pi / 4, -math.pi / 4),
                   (3 * math.pi / 4, math.pi / 4), (0, 0)]
        for index, answer in enumerate(answers):
            case = self.tangent_case(index)
            for got, want in zip(case["expected_derivative"], answer, strict=True):
                self.assertAlmostEqual(got, want, delta=1e-15)
            self.check_case(case)
            case["center"] = [-999999, 999999]
            self.check_case(case)
        dx, dy = answers[1]
        # At pi/4 on a 2-by-1 ellipse rotated pi/4, radius=(.5,1.5)
        # and rotated implicit gradient=(-.25,.75). Only the latter is normal.
        self.assertAlmostEqual(dx * -0.25 + dy * 0.75, 0, delta=1e-15)
        self.assertAlmostEqual(dx * 0.5 + dy * 1.5, -3 * math.pi / 4, delta=1e-15)

    def test_each_tangent_component_and_common_wrong_formulas_fail(self) -> None:
        for index in range(4):
            for axis in range(2):
                with self.subTest(index=index, axis=axis):
                    case = self.tangent_case(index)
                    self.check_case(case)
                    case["expected_derivative"][axis] += 1e-6
                    self.reject_case(case, schema=False)
        correct = self.tangent_case(1)["expected_derivative"]
        length = math.hypot(*correct)
        wrong = [[-x for x in correct], [x / (math.pi / 2) for x in correct],
                 [x / length for x in correct], [correct[0] + 3, correct[1] - 4],
                 [-math.pi / math.sqrt(2), math.pi / (2 * math.sqrt(2))]]
        for answer in wrong:
            case = self.tangent_case(1)
            case["expected_derivative"] = answer
            self.reject_case(case, schema=False)

    def test_tangent_output_bound_is_separate_and_absolute_only(self) -> None:
        for sweep in (0, math.tau, -math.tau):
            for t in (0, 1):
                case = self.tangent_case()
                case.update(rx=1_000_000, ry=1_000_000, sweep_angle=sweep, t=t)
                # End of a full turn has the floating-point sine residual.
                case["expected_derivative"] = [-sweep * 1_000_000 * math.sin(t * sweep),
                                               sweep * 1_000_000 * math.cos(t * sweep)]
                self.check_case(case)
                case["expected_derivative"][1] += 1e-6
                self.reject_case(case, schema=False)
        case = self.tangent_case()
        case["expected_derivative"] = [10_000_001, 0]
        self.reject_case(case)
        old = copy.deepcopy(self.document["cases"][0])
        old["point"] = [1_000_001, 0]
        self.reject_case(old)
        bezier = copy.deepcopy(self.document["cases"][13])
        bezier["expected_derivative"] = [1_000_001, 0]
        self.reject_case(bezier)

    def test_tangent_closed_shapes_and_domains(self) -> None:
        base = self.tangent_case()
        for key in base:
            case = copy.deepcopy(base)
            del case[key]
            self.reject_case(case)
        edits: list[dict[str, Any]] = [
            {"extra": 1}, {"center": [0]}, {"center": "0,0"},
            {"expected_derivative": [0, 1, 2]}, {"expected_derivative": None},
            {"rx": 0}, {"ry": -1}, {"rx": 1_000_001}, {"start_angle": 1_000_001},
            {"sweep_angle": math.nextafter(math.tau, math.inf)},
            {"sweep_angle": math.nextafter(-math.tau, -math.inf)},
            {"t": -1e-6}, {"t": 1.000001}, {"id": "Bad_id"},
            {"operation": "center-tangent"},
        ]
        for edit in edits:
            with self.subTest(edit=edit):
                case = copy.deepcopy(base)
                case.update(edit)
                self.reject_case(case)
        for key in ("rx", "ry", "start_angle", "sweep_angle", "x_rotation", "t"):
            for value in (True, "1", None, [], {}):
                case = copy.deepcopy(base)
                case[key] = value
                self.reject_case(case)

    def test_huge_integers_reject_without_float_conversion_overflow(self) -> None:
        for value in (10**400, -(10**400)):
            for key in ("rx", "ry", "start_angle", "sweep_angle", "x_rotation", "t"):
                case = self.tangent_case()
                case[key] = value
                self.reject_case(case)
            for key in ("center", "expected_derivative"):
                case = self.tangent_case()
                case[key][0] = value
                self.reject_case(case)
            old = copy.deepcopy(self.document["cases"][0])
            old["point"][0] = value
            self.reject_case(old)

    def test_tangent_nonfinite_inputs_outputs_and_derived_boundary(self) -> None:
        for value in (math.nan, math.inf, -math.inf):
            for key in ("rx", "ry", "start_angle", "sweep_angle", "x_rotation", "t"):
                case = self.tangent_case()
                case[key] = value
                self.reject_case(case, schema=False)
            for key in ("center", "expected_derivative"):
                case = self.tangent_case()
                case[key][0] = value
                self.reject_case(case, schema=False)
            with self.assertRaisesRegex(ValueError, "derived.*finite"):
                MODULE._expected([0, 0], (value, 0.0), "test-result")

    def test_root_caps_duplicate_tangent_transport_and_fixed_tolerance(self) -> None:
        base = self.tangent_case()
        for count in (1, 64):
            document = {"version": 1, "absolute_tolerance": 1e-12,
                        "cases": [dict(base, id=f"tangent-{i}") for i in range(count)]}
            self.assertEqual(MODULE.validate_document(document), count)
        for edit in ({"cases": [base] * 65}, {"cases": None}, {"version": True},
                     {"absolute_tolerance": True}, {"absolute_tolerance": 1e-11}):
            changed = copy.deepcopy(self.document)
            changed.update(edit)
            with self.assertRaises(ValueError):
                MODULE.validate_document(changed)
        raw = json.dumps(base).replace('"t": 0', '"t": 0, "t": 0')
        with self.assertRaisesRegex(ValueError, "duplicate JSON key"):
            MODULE.parse_json(raw)
        with self.assertRaisesRegex(ValueError, "duplicate case id"):
            MODULE.validate_document({"version": 1, "absolute_tolerance": 1e-12,
                                      "cases": [base, base]})


if __name__ == "__main__":
    unittest.main()
