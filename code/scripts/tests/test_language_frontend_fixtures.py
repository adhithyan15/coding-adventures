"""Contract tests for the shared Dart/Swift frontend fixture gate."""

from __future__ import annotations

import copy
import hashlib
import json
import sys
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator


SCRIPTS = Path(__file__).resolve().parents[1]
REPO = SCRIPTS.parents[1]
FIXTURES = REPO / "code/specs/fixtures/language-frontends-v1"
sys.path.insert(0, str(SCRIPTS))

from language_frontend_fixtures import (  # noqa: E402
    FAMILIES,
    validate_cases,
    validate_grammars,
)


def load(name: str) -> dict:
    return json.loads((FIXTURES / name).read_text(encoding="utf-8"))


class LanguageFrontendFixturesTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.schema = load("schema.json")
        cls.grammars = load("grammars.json")
        cls.cases = load("cases.json")

    def test_closed_schema_and_complete_roster(self) -> None:
        Draft202012Validator.check_schema(self.schema)
        for document in (self.grammars, self.cases):
            Draft202012Validator(self.schema).validate(document)
        self.assertEqual(12, len(FAMILIES))
        self.assertEqual(set(FAMILIES), {case["family"] for case in self.cases["cases"]})
        self.assertEqual(set(FAMILIES), {entry["family"] for entry in self.grammars["grammars"]})
        self.assertGreaterEqual(len(self.cases["cases"]), 12)

    def test_manifest_pins_every_canonical_grammar_byte(self) -> None:
        summary = validate_grammars(REPO, self.grammars)
        self.assertEqual(12, summary["families"])
        self.assertGreaterEqual(summary["grammar_files"], 24)
        for entry in self.grammars["grammars"]:
            path = REPO / entry["path"]
            self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), entry["sha256"])

    def test_each_family_has_a_complete_success_projection(self) -> None:
        summary = validate_cases(self.grammars, self.cases)
        self.assertEqual(12, summary["families"])
        for family in FAMILIES:
            successes = [case for case in self.cases["cases"]
                         if case["family"] == family and case["outcome"] == "ok"]
            self.assertTrue(successes, family)
            for case in successes:
                self.assertEqual("EOF", case["tokens"][-1]["type"])
                self.assertIn("rule", case["ast"])

    def test_rejects_stale_grammar_and_path_traversal(self) -> None:
        stale = copy.deepcopy(self.grammars)
        stale["grammars"][0]["sha256"] = "0" * 64
        with self.assertRaises(ValueError):
            validate_grammars(REPO, stale)
        escaped = copy.deepcopy(self.grammars)
        escaped["grammars"][0]["path"] = "../outside.tokens"
        with self.assertRaises(ValueError):
            validate_grammars(REPO, escaped)

    def test_rejects_duplicate_case_and_rewritten_expectation(self) -> None:
        duplicate = copy.deepcopy(self.cases)
        duplicate["cases"].append(copy.deepcopy(duplicate["cases"][0]))
        with self.assertRaises(ValueError):
            validate_cases(self.grammars, duplicate)
        wrong = copy.deepcopy(self.cases)
        wrong["cases"][0]["tokens"][-1]["type"] = "NOT_EOF"
        with self.assertRaises(ValueError):
            validate_cases(self.grammars, wrong)

    def test_rejects_unreferenced_or_unknown_edition(self) -> None:
        unknown = copy.deepcopy(self.cases)
        unknown["cases"][0]["version"] = "not-a-version"
        with self.assertRaises(ValueError):
            validate_cases(self.grammars, unknown)


if __name__ == "__main__":
    unittest.main()
