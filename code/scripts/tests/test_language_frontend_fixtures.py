"""Contract tests for the shared Dart/Swift frontend fixture gate."""

from __future__ import annotations

import copy
import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator

SCRIPTS = Path(__file__).resolve().parents[1]
REPO = SCRIPTS.parents[1]
FIXTURES = REPO / "code/specs/fixtures/language-frontends-v1"
sys.path.insert(0, str(SCRIPTS))

from language_frontend_fixtures import (
    FAMILIES,
    MAX_CASE_BYTES,
    validate_cases,
    validate_corpus,
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
        self.assertEqual(
            set(FAMILIES), {case["family"] for case in self.cases["cases"]}
        )
        self.assertEqual(
            set(FAMILIES), {entry["family"] for entry in self.grammars["grammars"]}
        )
        self.assertGreaterEqual(len(self.cases["cases"]), 12)

    def test_manifest_pins_every_canonical_grammar_byte(self) -> None:
        summary = validate_grammars(REPO, self.grammars)
        self.assertEqual(12, summary["families"])
        self.assertGreaterEqual(summary["grammar_files"], 24)
        self.assertTrue(
            any(
                entry["path"].startswith("code/grammars/ecmascript/")
                for entry in self.grammars["grammars"]
            )
        )
        for entry in self.grammars["grammars"]:
            path = REPO / entry["path"]
            self.assertEqual(
                hashlib.sha256(path.read_bytes()).hexdigest(), entry["sha256"]
            )

    def test_each_family_has_a_complete_success_projection(self) -> None:
        summary = validate_cases(self.grammars, self.cases)
        self.assertEqual(12, summary["families"])
        for family in FAMILIES:
            successes = [
                case
                for case in self.cases["cases"]
                if case["family"] == family and case["outcome"] == "ok"
            ]
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

    def test_rejects_duplicate_case_and_invalid_eof(self) -> None:
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

    def test_rejects_missing_family_and_unpaired_grammar(self) -> None:
        missing_case = copy.deepcopy(self.cases)
        missing_case["cases"] = [
            case for case in missing_case["cases"] if case["family"] != "vhdl"
        ]
        with self.assertRaises(ValueError):
            validate_cases(self.grammars, missing_case)
        missing_grammar = copy.deepcopy(self.grammars)
        missing_grammar["grammars"].pop()
        with self.assertRaises(ValueError):
            validate_grammars(REPO, missing_grammar)

    def test_rejects_invalid_ast_token_reference_and_open_case(self) -> None:
        invalid = copy.deepcopy(self.cases)
        invalid["cases"][0]["ast"] = {"token": len(invalid["cases"][0]["tokens"]) - 1}
        with self.assertRaises(ValueError):
            validate_cases(self.grammars, invalid)
        incomplete = copy.deepcopy(self.cases)
        incomplete["cases"][0]["ast"] = {"rule": "compilation_unit", "children": []}
        with self.assertRaises(ValueError):
            validate_cases(self.grammars, incomplete)
        opened = copy.deepcopy(self.cases)
        opened["cases"][0]["host_path"] = "C:/secret"
        with self.assertRaises(ValueError):
            validate_cases(self.grammars, opened)

    def test_entrypoint_validates_real_corpus(self) -> None:
        self.assertEqual(12, validate_corpus()["families"])

    def test_rejects_oversized_case_file_before_json_parse(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            fixtures = Path(temp)
            for name in ("schema.json", "grammars.json"):
                (fixtures / name).write_bytes((FIXTURES / name).read_bytes())
            (fixtures / "cases.json").write_bytes(b" " * (MAX_CASE_BYTES + 1))
            with self.assertRaises(ValueError):
                validate_corpus(REPO, fixtures)

    @unittest.skipIf(
        sys.version_info < (3, 12), "reference packages require Python 3.12+"
    )
    def test_checked_expectations_replay_canonical_python_engine(self) -> None:
        for package in (
            "grammar-tools",
            "lexer",
            "parser",
            "state-machine",
            "directed-graph",
            "graph",
        ):
            sys.path.insert(0, str(REPO / "code/packages/python" / package / "src"))
        from grammar_tools import parse_parser_grammar, parse_token_grammar
        from lang_parser.grammar_parser import ASTNode, GrammarParser
        from lexer.grammar_lexer import GrammarLexer
        from lexer.tokenizer import Token

        for case in self.cases["cases"]:
            directory = (
                REPO
                / "code/grammars"
                / (
                    "ecmascript"
                    if case["family"] == "javascript"
                    and case["version"] != "javascript"
                    else case["family"]
                )
            )
            version = case["version"]
            token_grammar = parse_token_grammar(
                (directory / f"{version}.tokens").read_text(encoding="utf-8")
            )
            parser_grammar = parse_parser_grammar(
                (directory / f"{version}.grammar").read_text(encoding="utf-8")
            )
            tokens = GrammarLexer(case["source"], token_grammar).tokenize()
            actual_tokens = [
                {
                    "type": getattr(token.type, "name", token.type),
                    "value": token.value,
                    "line": token.line,
                    "column": token.column,
                }
                for token in tokens
            ]
            self.assertEqual(case["tokens"], actual_tokens, case["id"])

            def project(node: ASTNode | Token, token_stream: list[Token]) -> dict:
                if isinstance(node, Token):
                    return {"token": token_stream.index(node)}
                return {
                    "rule": node.rule_name,
                    "children": [
                        project(child, token_stream) for child in node.children
                    ],
                }

            ast = GrammarParser(tokens, parser_grammar).parse()
            self.assertEqual(case["ast"], project(ast, tokens), case["id"])


if __name__ == "__main__":
    unittest.main()
