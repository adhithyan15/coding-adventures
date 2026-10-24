"""Falsify the portable GFM extension fixture independently of native parsers."""

from __future__ import annotations

import contextlib
import copy
import importlib.util
import io
import json
import pathlib
import runpy
import sys
import tempfile
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "code/scripts/gfm_extension_conformance.py"
CORPUS = ROOT / "code/specs/fixtures/gfm-extension-v1/cases.json"
SCHEMA = CORPUS.with_name("schema.json")
SPEC = importlib.util.spec_from_file_location("gfm_extension_conformance", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class GfmExtensionConformanceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.document = MODULE.load_corpus(CORPUS)

    def reject(self, document: object) -> None:
        with self.assertRaises(ValueError):
            MODULE.validate_document(document)

    def test_complete_corpus_and_strict_loader_validate(self) -> None:
        self.assertEqual(MODULE.validate_document(self.document), 16)
        self.assertEqual(MODULE.parse_json(CORPUS.read_bytes()), self.document)
        self.assertEqual(
            tuple(case["id"] for case in self.document["cases"]), MODULE.CASE_IDS
        )
        self.assertEqual(self.document["limits"], MODULE.LIMITS)

    def test_published_schema_closes_every_transport_and_node_object(self) -> None:
        schema = json.loads(SCHEMA.read_text("utf-8"))
        self.assertEqual(
            schema["$schema"], "https://json-schema.org/draft/2020-12/schema"
        )
        self.assertFalse(schema["additionalProperties"])
        self.assertEqual(schema["properties"]["cases"]["minItems"], 16)
        self.assertEqual(schema["properties"]["cases"]["maxItems"], 16)
        expected_defs = {
            "case",
            "document",
            "block",
            "paragraph",
            "inline",
            "text",
            "strikethrough",
            "strong",
            "soft_break",
            "list",
            "task_item",
            "table",
            "table_row",
            "table_cell",
            "fenced_block",
        }
        self.assertEqual(set(schema["$defs"]), expected_defs)
        for name, definition in schema["$defs"].items():
            if definition.get("type") == "object":
                self.assertFalse(definition["additionalProperties"], name)

    def test_each_source_ast_and_html_literal_is_independently_pinned(self) -> None:
        for index, original in enumerate(self.document["cases"]):
            for field in ("source", "expected_ast", "expected_html"):
                with self.subTest(case=original["id"], field=field):
                    changed = copy.deepcopy(self.document)
                    case = changed["cases"][index]
                    if field == "source":
                        case[field] += " "
                    elif field == "expected_html":
                        case[field] += "\n"
                    else:
                        case[field]["children"].append(
                            {"type": "paragraph", "children": []}
                        )
                    self.reject(changed)

    def test_case_roster_order_and_closed_fields_are_exact(self) -> None:
        for action in ("missing", "extra", "duplicate", "reordered", "nonlist"):
            changed = copy.deepcopy(self.document)
            cases = changed["cases"]
            if action == "missing":
                cases.pop()
            elif action == "extra":
                cases.append(copy.deepcopy(cases[-1]))
            elif action == "duplicate":
                cases[-1] = copy.deepcopy(cases[0])
            elif action == "reordered":
                cases[0], cases[1] = cases[1], cases[0]
            else:
                changed["cases"] = {}
            self.reject(changed)
        for target in (self.document, self.document["cases"][0]):
            changed = copy.deepcopy(self.document)
            changed_target = changed if target is self.document else changed["cases"][0]
            changed_target["extension"] = True
            self.reject(changed)

    def test_node_shapes_table_width_tasks_and_fences_are_checked(self) -> None:
        changed = copy.deepcopy(self.document)
        changed["cases"][0]["expected_ast"]["children"][0]["surprise"] = None
        self.reject(changed)

        changed = copy.deepcopy(self.document)
        table = changed["cases"][7]["expected_ast"]["children"][0]
        table["children"][1]["children"].pop()
        self.reject(changed)

        changed = copy.deepcopy(self.document)
        table = changed["cases"][7]["expected_ast"]["children"][0]
        table["children"][1]["isHeader"] = True
        self.reject(changed)

        changed = copy.deepcopy(self.document)
        task = changed["cases"][3]["expected_ast"]["children"][0]
        task["ordered"] = True
        self.reject(changed)

        changed = copy.deepcopy(self.document)
        fence = changed["cases"][12]["expected_ast"]["children"][0]
        fence["name"] = "javascript"
        self.reject(changed)

        changed = copy.deepcopy(self.document)
        fence = changed["cases"][12]["expected_ast"]["children"][0]
        fence["value"] = "no-final-line-feed"
        self.reject(changed)

    def test_types_and_resource_bounds_are_not_coerced(self) -> None:
        for value in (True, 1.0, "1", 0, 2, None):
            changed = copy.deepcopy(self.document)
            changed["schema_version"] = value
            self.reject(changed)
        changed = copy.deepcopy(self.document)
        changed["limits"]["max_ast_nodes"] = 257
        self.reject(changed)
        changed = copy.deepcopy(self.document)
        changed["cases"][0]["source"] = "x" * 1025 + "\n"
        self.reject(changed)
        changed = copy.deepcopy(self.document)
        changed["cases"][0]["expected_html"] = "x" * 32769
        self.reject(changed)
        changed = copy.deepcopy(self.document)
        changed["cases"][0]["expected_ast"]["children"] = [
            {"type": "paragraph", "children": []} for _ in range(257)
        ]
        self.reject(changed)

    def test_every_validator_refusal_family_is_exercised(self) -> None:
        nested: object = 0
        for _ in range(17):
            nested = [nested]
        with self.assertRaisesRegex(ValueError, "JSON container depth"):
            MODULE._guard_tree(nested)
        with self.assertRaisesRegex(ValueError, "expected text"):
            MODULE._text("", "empty", 8, empty=False)
        with self.assertRaisesRegex(ValueError, "inline node"):
            MODULE._validate_inline([], {"nodes": 0}, 1, "inline")
        with self.assertRaisesRegex(ValueError, "unsupported inline"):
            MODULE._validate_inline({"type": "unknown"}, {"nodes": 0}, 1, "inline")
        with self.assertRaisesRegex(ValueError, "node count"):
            MODULE._count_node({"nodes": 256}, 1)
        with self.assertRaisesRegex(ValueError, "AST depth"):
            MODULE._count_node({"nodes": 0}, 17)
        with self.assertRaisesRegex(ValueError, "expected paragraph"):
            MODULE._validate_paragraph(
                {"type": "heading", "children": []}, {"nodes": 0}, 1, "block"
            )
        with self.assertRaisesRegex(ValueError, "invalid alignment"):
            MODULE._validate_table(
                {"align": ["diagonal"], "children": []},
                {"nodes": 0},
                1,
                "table",
            )
        bad_row = {
            "align": [None],
            "children": [
                {"type": "row", "isHeader": True, "children": []},
                {"type": "table_row", "isHeader": False, "children": []},
            ],
        }
        with self.assertRaisesRegex(ValueError, "invalid table row"):
            MODULE._validate_table(bad_row, {"nodes": 0}, 1, "table")
        bad_cell = {
            "align": [None],
            "children": [
                {
                    "type": "table_row",
                    "isHeader": True,
                    "children": [{"type": "cell", "children": []}],
                },
                {
                    "type": "table_row",
                    "isHeader": False,
                    "children": [{"type": "table_cell", "children": []}],
                },
            ],
        }
        with self.assertRaisesRegex(ValueError, "invalid table cell"):
            MODULE._validate_table(bad_cell, {"nodes": 0}, 1, "table")
        with self.assertRaisesRegex(ValueError, "block node"):
            MODULE._validate_block([], {"nodes": 0}, 1, "block")
        with self.assertRaisesRegex(ValueError, "list child"):
            MODULE._validate_block(
                {
                    "type": "list",
                    "ordered": False,
                    "start": None,
                    "tight": True,
                    "children": [
                        {"type": "list_item", "checked": False, "children": []}
                    ],
                },
                {"nodes": 0},
                1,
                "block",
            )
        with self.assertRaisesRegex(ValueError, "nullability"):
            MODULE._validate_block(
                {"type": "fenced_block", "name": None, "info": "x", "value": ""},
                {"nodes": 0},
                1,
                "block",
            )
        with self.assertRaisesRegex(ValueError, "unsupported block"):
            MODULE._validate_block({"type": "heading"}, {"nodes": 0}, 1, "block")
        with self.assertRaisesRegex(ValueError, "document root"):
            MODULE._validate_ast({"type": "page", "children": []}, "ast")
        changed = copy.deepcopy(self.document)
        changed["profile"] = "gfm-extension-v2"
        self.reject(changed)
        changed = copy.deepcopy(self.document)
        changed["cases"][0]["source"] = "x\n" * 129
        self.reject(changed)

    def test_parser_rejects_ambiguous_or_hostile_json_transport(self) -> None:
        for raw in (
            b'{"a":1,"a":2}',
            b'{"a":1,"\\u0061":2}',
            b"NaN",
            b"Infinity",
            b"1.0",
            b"1e0",
            b"99999999999",
            b"{} {}",
            b"{",
            b'"\\ud800"',
            b"\xff",
            b"\xef\xbb\xbf{}",
            b"[" * 17 + b"0" + b"]" * 17,
        ):
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                MODULE.parse_json(raw)
        self.assertEqual(
            MODULE.parse_json(b"[" * 16 + b"0" + b"]" * 16),
            [[[[[[[[[[[[[[[[0]]]]]]]]]]]]]]]],
        )

    def test_loader_and_cli_bound_reads_and_report_count(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            too_large = pathlib.Path(directory) / "too-large.json"
            too_large.write_bytes(b" " * (MODULE.MAX_BYTES + 1))
            with self.assertRaises(ValueError):
                MODULE.load_corpus(too_large)
            with self.assertRaises(OSError):
                MODULE.load_corpus(pathlib.Path(directory) / "missing.json")

        for args in ([], ["--fixture", str(CORPUS)]):
            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout):
                self.assertEqual(MODULE.main(args), 0)
            self.assertEqual(
                stdout.getvalue(), "validated 16 portable GFM extension cases\n"
            )

        stdout = io.StringIO()
        with (
            contextlib.redirect_stdout(stdout),
            mock.patch.object(sys, "argv", [str(SCRIPT)]),
        ):
            with self.assertRaises(SystemExit) as exit_result:
                runpy.run_path(str(SCRIPT), run_name="__main__")
        self.assertEqual(exit_result.exception.code, 0)
        self.assertEqual(
            stdout.getvalue(), "validated 16 portable GFM extension cases\n"
        )


if __name__ == "__main__":
    unittest.main()
