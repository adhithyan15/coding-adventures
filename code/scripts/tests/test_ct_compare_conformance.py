"""Exercise the CT01 fixture guard independently of language adapters."""

from __future__ import annotations

import contextlib
import copy
import importlib.util
import io
import json
import pathlib
import sys
import unittest
from unittest import mock

from jsonschema import Draft202012Validator

ROOT = pathlib.Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "code" / "scripts" / "ct_compare_conformance.py"
CORPUS = ROOT / "code" / "specs" / "fixtures" / "ct-compare-v1" / "cases.json"
SCHEMA = ROOT / "code" / "specs" / "fixtures" / "ct-compare-v1" / "schema.json"
SPEC = importlib.util.spec_from_file_location("ct_compare_conformance", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class CtCompareConformanceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.document = json.loads(CORPUS.read_text(encoding="utf-8"))

    def test_canonical_corpus_and_operation_coverage(self) -> None:
        self.assertEqual(MODULE.validate_document(self.document), 21)
        self.assertEqual(
            MODULE.validate_document(
                MODULE.parse_json(CORPUS.read_text(encoding="utf-8"))
            ),
            21,
        )
        cases = self.document["cases"]
        self.assertEqual(
            {case["operation"] for case in cases},
            {"eq", "eq_fixed", "select_bytes", "eq_u64"},
        )
        self.assertEqual(sum(case["id"].startswith("u64-") for case in cases), 5)
        self.assertEqual(
            next(case for case in cases if case["id"] == "eq-256-byte-last-different")[
                "expected_work"
            ]["byte_positions"],
            256,
        )

    def test_published_schema_is_closed_and_accepts_corpus(self) -> None:
        schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
        Draft202012Validator.check_schema(schema)
        validator = Draft202012Validator(schema)
        self.assertEqual(list(validator.iter_errors(self.document)), [])
        changed = copy.deepcopy(self.document)
        changed["cases"][0]["unreviewed"] = True
        self.assertTrue(list(validator.iter_errors(changed)))

    def test_duplicate_id_and_extra_field_fail_closed(self) -> None:
        changed = copy.deepcopy(self.document)
        changed["cases"][1]["id"] = changed["cases"][0]["id"]
        with self.assertRaisesRegex(ValueError, "duplicate case id"):
            MODULE.validate_document(changed)
        changed = copy.deepcopy(self.document)
        changed["cases"][0]["unreviewed"] = True
        with self.assertRaisesRegex(ValueError, "fields"):
            MODULE.validate_document(changed)

    def test_expectations_are_recomputed(self) -> None:
        changed = copy.deepcopy(self.document)
        changed["cases"][2]["expected"]["equal"] = True
        with self.assertRaisesRegex(ValueError, "expected result"):
            MODULE.validate_document(changed)
        changed = copy.deepcopy(self.document)
        changed["cases"][2]["expected_work"]["byte_reads"] = 0
        with self.assertRaisesRegex(ValueError, "expected_work"):
            MODULE.validate_document(changed)

    def test_hex_and_unsigned_width_are_canonical(self) -> None:
        for bad in ("A0", "a", "0x00", "g0"):
            with self.subTest(bad=bad):
                changed = copy.deepcopy(self.document)
                changed["cases"][1]["left_hex"] = bad
                with self.assertRaisesRegex(ValueError, "hex"):
                    MODULE.validate_document(changed)
        changed = copy.deepcopy(self.document)
        changed["cases"][-1]["left_u64"] = "0"
        with self.assertRaisesRegex(ValueError, "u64"):
            MODULE.validate_document(changed)

    def test_fixed_width_and_select_error_are_checked(self) -> None:
        changed = copy.deepcopy(self.document)
        fixed = next(
            case for case in changed["cases"] if case["id"] == "fixed-16-equal"
        )
        fixed["fixed_size_bytes"] = 15
        with self.assertRaisesRegex(ValueError, "fixed_size_bytes"):
            MODULE.validate_document(changed)
        changed = copy.deepcopy(self.document)
        select = next(
            case
            for case in changed["cases"]
            if case["id"] == "select-public-length-mismatch"
        )
        select["expected"] = {"output_hex": "00", "fresh_output": True}
        with self.assertRaisesRegex(ValueError, "expected result"):
            MODULE.validate_document(changed)

    def test_duplicate_json_keys_rejected_before_validation(self) -> None:
        with self.assertRaisesRegex(ValueError, "duplicate JSON key"):
            MODULE.parse_json('{"schema_version":1,"schema_version":1}')

    def test_closed_profile_case_id_operation_and_choice(self) -> None:
        for field, bad, message in (
            ("schema_version", True, "schema_version"),
            ("profile", "ct-compare-v2", "profile"),
            ("cases", [], "cases"),
        ):
            with self.subTest(field=field):
                changed = copy.deepcopy(self.document)
                changed[field] = bad
                with self.assertRaisesRegex(ValueError, message):
                    MODULE.validate_document(changed)
        for field, bad, message in (
            ("operation", "secret-branch", "unknown operation"),
            ("id", "Not-Canonical", "case id"),
        ):
            with self.subTest(field=field):
                changed = copy.deepcopy(self.document)
                changed["cases"][0][field] = bad
                with self.assertRaisesRegex(ValueError, message):
                    MODULE.validate_document(changed)
        changed = copy.deepcopy(self.document)
        changed["cases"][0] = None
        with self.assertRaisesRegex(TypeError, "case fields"):
            MODULE.validate_document(changed)
        changed = copy.deepcopy(self.document)
        select = next(
            case for case in changed["cases"] if case["id"] == "select-left-fresh"
        )
        select["choice"] = 1
        with self.assertRaisesRegex(ValueError, "choice"):
            MODULE.validate_document(changed)

    def test_cli_main_reports_validated_count(self) -> None:
        output = io.StringIO()
        with (
            mock.patch.object(sys, "argv", ["ct_compare_conformance.py"]),
            contextlib.redirect_stdout(output),
        ):
            MODULE.main()
        self.assertEqual(output.getvalue(), "ct-compare-v1: 21 cases valid\n")


if __name__ == "__main__":
    unittest.main()
