"""Falsify the portable CommonMark foundation without running a native parser."""

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
SCRIPT = ROOT / "code/scripts/commonmark_conformance.py"
FIXTURES = ROOT / "code/specs/fixtures/commonmark-0.31.2"
CORPUS = FIXTURES / "spec.json"
RENDERER = FIXTURES / "renderer-cases.json"
PROVENANCE = FIXTURES / "provenance.json"
SPEC = importlib.util.spec_from_file_location("commonmark_conformance", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class CommonmarkConformanceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.corpus = MODULE.load_corpus(CORPUS)
        self.renderer = MODULE.load_renderer(RENDERER)
        self.provenance = MODULE.load_provenance(PROVENANCE)

    def reject_corpus(self, document: object) -> None:
        with self.assertRaises(ValueError):
            MODULE.validate_corpus(document)

    def reject_renderer(self, document: object) -> None:
        with self.assertRaises(ValueError):
            MODULE.validate_renderer(document)

    def test_complete_foundation_validates(self) -> None:
        self.assertEqual(MODULE.validate_corpus(self.corpus), 652)
        self.assertEqual(MODULE.validate_renderer(self.renderer), 16)
        self.assertEqual(MODULE.validate_provenance(self.provenance), 1)
        self.assertEqual(MODULE.validate_all(), (652, 16))
        self.assertEqual(
            MODULE._sha256(CORPUS.read_bytes()), MODULE.OFFICIAL_RAW_SHA256
        )
        self.assertEqual(CORPUS.stat().st_size, MODULE.OFFICIAL_RAW_BYTES)

    def test_published_schemas_are_closed(self) -> None:
        expected = {
            "corpus-schema.json": {"example"},
            "provenance-schema.json": {"license"},
            "renderer-schema.json": {
                "case",
                "document",
                "block",
                "inline",
                "heading",
                "paragraph",
                "code_block",
                "blockquote",
                "list",
                "list_item",
                "thematic_break",
                "raw_block",
                "text",
                "emphasis",
                "strong",
                "code_span",
                "link",
                "image",
                "autolink",
                "raw_inline",
                "hard_break",
                "soft_break",
            },
        }
        for filename, definitions in expected.items():
            with self.subTest(filename=filename):
                schema = json.loads((FIXTURES / filename).read_text("utf-8"))
                self.assertEqual(
                    schema["$schema"], "https://json-schema.org/draft/2020-12/schema"
                )
                if schema.get("type") == "object":
                    self.assertFalse(schema["additionalProperties"])
                self.assertEqual(set(schema["$defs"]), definitions)
                for name, definition in schema["$defs"].items():
                    if definition.get("type") == "object":
                        self.assertFalse(definition["additionalProperties"], name)

    def test_each_official_field_is_independently_pinned(self) -> None:
        for field in (
            "markdown",
            "html",
            "example",
            "start_line",
            "end_line",
            "section",
        ):
            with self.subTest(field=field):
                changed = copy.deepcopy(self.corpus)
                value = changed[0][field]
                changed[0][field] = value + " " if isinstance(value, str) else value + 1
                self.reject_corpus(changed)

    def test_official_roster_metadata_and_types_are_strict(self) -> None:
        for action in ("missing", "extra", "reordered", "duplicate", "nonlist"):
            changed = copy.deepcopy(self.corpus)
            if action == "missing":
                changed.pop()
            elif action == "extra":
                changed.append(copy.deepcopy(changed[-1]))
            elif action == "reordered":
                changed[0], changed[1] = changed[1], changed[0]
            elif action == "duplicate":
                changed[1] = copy.deepcopy(changed[0])
            else:
                changed = {}
            self.reject_corpus(changed)

        for field, value in (
            ("example", True),
            ("start_line", 0),
            ("end_line", 355),
            ("section", ""),
            ("markdown", 1),
            ("html", None),
        ):
            with self.subTest(field=field, value=value):
                changed = copy.deepcopy(self.corpus)
                changed[0][field] = value
                self.reject_corpus(changed)

        changed = copy.deepcopy(self.corpus)
        changed[0]["extra"] = True
        self.reject_corpus(changed)
        changed = copy.deepcopy(self.corpus)
        changed[1]["start_line"] = changed[0]["end_line"]
        self.reject_corpus(changed)

    def test_each_renderer_ast_option_and_html_is_independently_pinned(self) -> None:
        for index, original in enumerate(self.renderer["cases"]):
            for field in ("ast", "suppress_raw_html", "expected_html"):
                with self.subTest(case=original["id"], field=field):
                    changed = copy.deepcopy(self.renderer)
                    case = changed["cases"][index]
                    if field == "ast":
                        case[field]["children"].append(
                            {"type": "paragraph", "children": []}
                        )
                    elif field == "suppress_raw_html":
                        case[field] = not case[field]
                    else:
                        case[field] += "\n"
                    self.reject_renderer(changed)

    def test_renderer_roster_limits_and_closed_fields_are_exact(self) -> None:
        self.assertEqual(
            tuple(case["id"] for case in self.renderer["cases"]),
            MODULE.RENDERER_CASE_IDS,
        )
        self.assertEqual(self.renderer["limits"], MODULE.RENDERER_LIMITS)
        for action in ("missing", "extra", "reordered", "duplicate", "nonlist"):
            changed = copy.deepcopy(self.renderer)
            cases = changed["cases"]
            if action == "missing":
                cases.pop()
            elif action == "extra":
                cases.append(copy.deepcopy(cases[-1]))
            elif action == "reordered":
                cases[0], cases[1] = cases[1], cases[0]
            elif action == "duplicate":
                cases[-1] = copy.deepcopy(cases[0])
            else:
                changed["cases"] = {}
            self.reject_renderer(changed)

        for target in ("root", "limits", "case"):
            changed = copy.deepcopy(self.renderer)
            if target == "root":
                changed["extra"] = None
            elif target == "limits":
                changed["limits"]["extra"] = 1
            else:
                changed["cases"][0]["extra"] = None
            self.reject_renderer(changed)

    def test_renderer_vectors_cover_every_te00_node_kind(self) -> None:
        kinds: set[str] = set()
        pending: list[object] = [case["ast"] for case in self.renderer["cases"]]
        while pending:
            value = pending.pop()
            if isinstance(value, dict):
                if isinstance(value.get("type"), str):
                    kinds.add(value["type"])
                pending.extend(value.values())
            elif isinstance(value, list):
                pending.extend(value)
        self.assertEqual(kinds, MODULE.TE00_NODE_TYPES)

    def test_renderer_node_shapes_and_containment_are_strict(self) -> None:
        mutations = []

        bad = copy.deepcopy(self.renderer)
        bad["cases"][1]["ast"]["children"][0]["level"] = 7
        mutations.append(bad)

        bad = copy.deepcopy(self.renderer)
        bad["cases"][5]["ast"]["children"][0]["children"].append(
            {"type": "text", "value": "not a block"}
        )
        mutations.append(bad)

        bad = copy.deepcopy(self.renderer)
        bad["cases"][6]["ast"]["children"][0]["children"].append(
            {"type": "paragraph", "children": []}
        )
        mutations.append(bad)

        bad = copy.deepcopy(self.renderer)
        bad["cases"][9]["ast"]["children"][0]["children"][0]["children"] = [
            {
                "type": "link",
                "destination": "/nested",
                "title": None,
                "children": [{"type": "text", "value": "nested"}],
            }
        ]
        mutations.append(bad)

        bad = copy.deepcopy(self.renderer)
        bad["cases"][0]["ast"]["children"] = [{"type": "unknown"}]
        mutations.append(bad)

        for changed in mutations:
            self.reject_renderer(changed)

    def test_provenance_is_exact_and_closed(self) -> None:
        self.assertEqual(
            self.provenance["artifact"]["sha256"], MODULE.OFFICIAL_RAW_SHA256
        )
        self.assertEqual(
            self.provenance["artifact"]["bytes"], MODULE.OFFICIAL_RAW_BYTES
        )
        for field in (
            "profile",
            "source_url",
            "specification_url",
            "version",
            "published",
            "author",
        ):
            with self.subTest(field=field):
                changed = copy.deepcopy(self.provenance)
                changed[field] += "changed"
                with self.assertRaises(ValueError):
                    MODULE.validate_provenance(changed)
        changed = copy.deepcopy(self.provenance)
        changed["extra"] = True
        with self.assertRaises(ValueError):
            MODULE.validate_provenance(changed)

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
            b"[" * 33 + b"0" + b"]" * 33,
        ):
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                MODULE.parse_json(raw, MODULE.MAX_CORPUS_BYTES)
        self.assertEqual(
            MODULE.parse_json(b"[" * 32 + b"0" + b"]" * 32, 128),
            [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[0]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]],
        )

    def test_direct_object_guard_rejects_aliases_cycles_and_bounded_work(self) -> None:
        leaf: list[object] = []
        alias_dag: object = leaf
        for _ in range(12):
            alias_dag = [alias_dag] * 6
        with self.assertRaisesRegex(ValueError, "must be a tree"):
            MODULE._guard_tree(alias_dag)

        cycle: list[object] = []
        cycle.append(cycle)
        with self.assertRaisesRegex(ValueError, "must be a tree"):
            MODULE._guard_tree(cycle)

        with self.assertRaisesRegex(ValueError, "items"):
            MODULE._guard_tree([None] * MODULE.MAX_TREE_ITEMS)
        with self.assertRaisesRegex(ValueError, "text"):
            MODULE._guard_tree("x" * (MODULE.MAX_TREE_TEXT_BYTES + 1))
        with self.assertRaisesRegex(ValueError, "text"):
            MODULE._guard_tree("é" * (MODULE.MAX_TREE_TEXT_BYTES // 2 + 1))

    def test_each_validator_refusal_family_is_exercised(self) -> None:
        nested: object = 0
        for _ in range(MODULE.MAX_JSON_DEPTH + 1):
            nested = [nested]
        with self.assertRaisesRegex(ValueError, "depth"):
            MODULE._guard_tree(nested)

        for raw, ceiling in (("[]", 8), (b"[]", 0), (b"[]", True), (b"[]", 1)):
            with self.subTest(raw=raw, ceiling=ceiling), self.assertRaises(ValueError):
                MODULE.parse_json(raw, ceiling)

        with tempfile.TemporaryDirectory() as directory:
            changed = pathlib.Path(directory) / "spec.json"
            changed.write_bytes(b"[]")
            with self.assertRaisesRegex(ValueError, "digest"):
                MODULE.load_corpus(changed)

        with self.assertRaisesRegex(ValueError, "text exceeds"):
            MODULE._text("123456789", "text", 8)
        with self.assertRaisesRegex(ValueError, "node count"):
            MODULE._count_node({"nodes": 256, "kinds": set()}, 1, "text")
        with self.assertRaisesRegex(ValueError, "AST depth"):
            MODULE._count_node({"nodes": 0, "kinds": set()}, 33, "text")
        with self.assertRaisesRegex(ValueError, "string type"):
            MODULE._node([], "node")
        with self.assertRaisesRegex(ValueError, "expected boolean"):
            MODULE._validate_inline(
                {"type": "autolink", "destination": "/", "isEmail": 0},
                {"nodes": 0, "kinds": set()},
                1,
                "inline",
            )
        with self.assertRaisesRegex(ValueError, "unsupported inline"):
            MODULE._validate_inline(
                {"type": "unknown"},
                {"nodes": 0, "kinds": set()},
                1,
                "inline",
            )
        for value, message in (
            (
                {
                    "type": "list",
                    "ordered": 0,
                    "start": None,
                    "tight": True,
                    "children": [],
                },
                "flags",
            ),
            (
                {
                    "type": "list",
                    "ordered": False,
                    "start": 1,
                    "tight": True,
                    "children": [],
                },
                "start must be null",
            ),
        ):
            with self.assertRaisesRegex(ValueError, message):
                MODULE._validate_block(value, {"nodes": 0, "kinds": set()}, 1, "block")
        with self.assertRaisesRegex(ValueError, "root type"):
            MODULE._validate_document({"type": "page", "children": []}, "document")

        changed = copy.deepcopy(self.renderer)
        changed["profile"] = "commonmark-renderer-v2"
        self.reject_renderer(changed)
        changed = copy.deepcopy(self.renderer)
        changed["cases"][0]["suppress_raw_html"] = 0
        self.reject_renderer(changed)
        with (
            mock.patch.object(MODULE, "TE00_NODE_TYPES", set()),
            self.assertRaisesRegex(ValueError, "every TE00"),
        ):
            MODULE.validate_renderer(self.renderer)

    def test_loaders_bound_reads_before_decode_or_digest(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            for loader, ceiling in (
                (MODULE.load_corpus, MODULE.MAX_CORPUS_BYTES),
                (MODULE.load_renderer, MODULE.MAX_RENDERER_BYTES),
                (MODULE.load_provenance, MODULE.MAX_PROVENANCE_BYTES),
            ):
                with self.subTest(loader=loader.__name__):
                    too_large = root / f"{loader.__name__}.json"
                    too_large.write_bytes(b" " * (ceiling + 1))
                    with self.assertRaises(ValueError):
                        loader(too_large)
            with self.assertRaises(OSError):
                MODULE.load_corpus(root / "missing.json")

    def test_cli_reports_both_fixture_counts(self) -> None:
        stdout = io.StringIO()
        with contextlib.redirect_stdout(stdout):
            self.assertEqual(MODULE.main([]), 0)
        self.assertEqual(
            stdout.getvalue(),
            "validated 652 CommonMark 0.31.2 examples and 16 renderer cases\n",
        )

        stdout = io.StringIO()
        with (
            contextlib.redirect_stdout(stdout),
            mock.patch.object(sys, "argv", [str(SCRIPT)]),
            self.assertRaises(SystemExit) as exit_result,
        ):
            runpy.run_path(str(SCRIPT), run_name="__main__")
        self.assertEqual(exit_result.exception.code, 0)
        self.assertEqual(
            stdout.getvalue(),
            "validated 652 CommonMark 0.31.2 examples and 16 renderer cases\n",
        )


if __name__ == "__main__":
    unittest.main()
