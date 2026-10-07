"""The neutral geometry oracle must reject corrupt or weakened cases."""

from __future__ import annotations

import copy
import importlib.util
import json
import pathlib
import unittest
from typing import Any, Callable

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
        self.assertEqual(MODULE.validate_document(self.document), 8)
        self.assertEqual(
            MODULE.validate_document(
                MODULE.parse_json(CORPUS.read_text(encoding="utf-8"))
            ),
            8,
        )

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
