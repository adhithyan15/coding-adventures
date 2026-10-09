"""The G2D02 flattening corpus is an independent, closed numerical contract."""

from __future__ import annotations

import copy
import importlib.util
import io
import json
import pathlib
import unittest
from contextlib import redirect_stdout

from jsonschema import Draft202012Validator  # type: ignore[import-untyped]

ROOT = pathlib.Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "code" / "scripts" / "bezier2d_flattening_conformance.py"
CORPUS = ROOT / "code" / "specs" / "fixtures" / "bezier2d-flattening-v1" / "cases.json"
SCHEMA = CORPUS.with_name("schema.json")
SPEC = importlib.util.spec_from_file_location("bezier2d_flattening_conformance", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class Bezier2DFlatteningConformanceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.document = MODULE.parse_json(CORPUS.read_text(encoding="utf-8"))

    def case(self, case_id: str) -> dict[str, object]:
        return next(case for case in self.document["cases"] if case["id"] == case_id)

    def test_corpus_and_closed_schema(self) -> None:
        schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
        Draft202012Validator.check_schema(schema)
        self.assertEqual(
            list(Draft202012Validator(schema).iter_errors(self.document)), []
        )
        self.assertEqual(MODULE.validate_document(self.document), 9)
        self.assertEqual(
            {case["id"] for case in self.document["cases"]},
            {
                "quadratic-straight",
                "quadratic-arch",
                "cubic-collinear-inside",
                "cubic-symmetric-s",
                "cubic-collinear-overshoot",
                "cubic-coincident-loop",
                "zero-tolerance",
                "negative-tolerance",
                "quadratic-budget-exhaustion",
            },
        )

    def test_witness_and_disposition_mutations_rejected(self) -> None:
        changed = copy.deepcopy(self.document)
        changed["cases"][3]["expected_witness"][1] = 0
        with self.assertRaisesRegex(ValueError, "witness"):
            MODULE.validate_document(changed)
        changed = copy.deepcopy(self.document)
        changed["cases"][4]["expected_witness_distance"] = 0
        with self.assertRaisesRegex(ValueError, "distance"):
            MODULE.validate_document(changed)
        changed = copy.deepcopy(self.document)
        changed["cases"][3]["expected_disposition"] = "chord"
        with self.assertRaisesRegex(ValueError, "disposition"):
            MODULE.validate_document(changed)

    def test_invalid_tolerance_boundaries_rejected(self) -> None:
        changed = copy.deepcopy(self.document)
        changed["cases"][6]["tolerance"] = 0.1
        with self.assertRaisesRegex(ValueError, "invalid tolerance"):
            MODULE.validate_document(changed)
        changed = copy.deepcopy(self.document)
        changed["cases"][0]["tolerance"] = 0
        with self.assertRaisesRegex(ValueError, "positive tolerance"):
            MODULE.validate_document(changed)
        with self.assertRaisesRegex(ValueError, "nonfinite JSON constant"):
            MODULE.parse_json('{"tolerance":NaN}')
        with self.assertRaisesRegex(ValueError, "nonfinite JSON constant"):
            MODULE.parse_json('{"tolerance":Infinity}')

    def test_shape_ids_and_limits_rejected(self) -> None:
        edits = [
            lambda d: d.update(version=2),
            lambda d: d.update(max_depth=33),
            lambda d: d.update(max_subdivisions=65_536),
            lambda d: d["cases"][0].update(control_points=[[0, 0], [1, 0]]),
            lambda d: d["cases"][0].update(extra=True),
            lambda d: d["cases"][0].update(id="Bad_ID"),
            lambda d: d["cases"][0].update(tolerance=float("nan")),
            lambda d: d["cases"][1]["control_points"][1].__setitem__(1, 1_000_001),
        ]
        for edit in edits:
            with self.subTest(edit=edit):
                changed = copy.deepcopy(self.document)
                edit(changed)
                with self.assertRaises(ValueError):
                    MODULE.validate_document(changed)
        changed = copy.deepcopy(self.document)
        changed["cases"][1]["id"] = changed["cases"][0]["id"]
        with self.assertRaisesRegex(ValueError, "duplicate case id"):
            MODULE.validate_document(changed)
        with self.assertRaisesRegex(ValueError, "duplicate JSON key"):
            MODULE.parse_json('{"version":1,"version":1}')
        schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
        changed = copy.deepcopy(self.document)
        changed["cases"][0]["id"] += "\n"
        self.assertNotEqual(list(Draft202012Validator(schema).iter_errors(changed)), [])

    def test_invalid_case_shapes_and_witness_boundaries(self) -> None:
        changed = copy.deepcopy(self.document)
        changed["cases"][0] = None
        with self.assertRaisesRegex(TypeError, "object"):
            MODULE.validate_document(changed)
        changed = copy.deepcopy(self.document)
        changed["cases"][0]["degree"] = 4
        with self.assertRaisesRegex(ValueError, "degree"):
            MODULE.validate_document(changed)
        changed = copy.deepcopy(self.document)
        changed["cases"][0]["expected_witness"] = [0.25]
        with self.assertRaisesRegex(ValueError, "point"):
            MODULE.validate_document(changed)
        changed = copy.deepcopy(self.document)
        changed["cases"][0]["witness_t"] = 2
        with self.assertRaisesRegex(ValueError, "witness_t"):
            MODULE.validate_document(changed)
        changed = copy.deepcopy(self.document)
        changed["cases"][0]["expected_disposition"] = "unreviewed"
        with self.assertRaisesRegex(ValueError, "unsupported"):
            MODULE.validate_document(changed)
        changed = copy.deepcopy(self.document)
        changed["cases"][3]["witness_t"] = 0.5
        changed["cases"][3]["expected_witness"] = [0, 0]
        changed["cases"][3]["expected_witness_distance"] = 0
        with self.assertRaisesRegex(ValueError, "witness distance"):
            MODULE.validate_document(changed)

    def test_root_bounds_and_cli_message(self) -> None:
        changed = copy.deepcopy(self.document)
        changed["absolute_tolerance"] = 0.1
        with self.assertRaisesRegex(ValueError, "absolute_tolerance"):
            MODULE.validate_document(changed)
        changed = copy.deepcopy(self.document)
        changed["cases"] = []
        with self.assertRaisesRegex(ValueError, "cases"):
            MODULE.validate_document(changed)
        output = io.StringIO()
        with redirect_stdout(output):
            MODULE.main()
        self.assertEqual(
            output.getvalue(), "validated 9 neutral G2D02 flattening cases\n"
        )

    def test_budget_error_has_independent_lower_bound(self) -> None:
        schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
        for field, value in (
            ("tolerance", 1e-9),
            ("expected_termination", "success"),
            ("control_points", [[0, 0], [0.5, 0.5], [1, 0]]),
            ("expected_disposition", "chord"),
        ):
            with self.subTest(field=field):
                changed = copy.deepcopy(self.document)
                changed["cases"][8][field] = value
                self.assertNotEqual(
                    list(Draft202012Validator(schema).iter_errors(changed)), []
                )
                with self.assertRaises(ValueError):
                    MODULE.validate_document(changed)


if __name__ == "__main__":
    unittest.main()
