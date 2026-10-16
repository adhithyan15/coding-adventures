"""Falsify STP01 independently of native providers and fixture generation."""

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

from jsonschema import Draft202012Validator

ROOT = pathlib.Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "code/scripts/starlark_opcode_provider_conformance.py"
CORPUS = ROOT / "code/specs/fixtures/starlark-opcode-provider-v1/cases.json"
SCHEMA = CORPUS.with_name("schema.json")
SPEC = importlib.util.spec_from_file_location("starlark_opcode_provider", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class OpcodeProviderConformanceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.document = json.loads(CORPUS.read_bytes())

    def reject(self, document: object) -> None:
        with self.assertRaises(ValueError):
            MODULE.validate_document(document)

    def test_canonical_complete_domain_and_map_counts(self) -> None:
        self.assertEqual(MODULE.validate_document(self.document), 359)
        self.assertEqual(MODULE.load_corpus(CORPUS), self.document)
        self.assertEqual(MODULE.parse_json(CORPUS.read_bytes()), self.document)
        rows = self.document["decode"]
        self.assertEqual([r["byte"] for r in rows], list(range(256)))
        self.assertEqual(sum(r["symbol"] is None for r in rows), 195)
        self.assertEqual(rows[0], {"byte": 0, "symbol": None, "category": None})
        self.assertEqual(rows[255]["symbol"], "Halt")
        self.assertEqual(
            {k: len(v) for k, v in self.document["operators"].items()},
            {"binary": 12, "compare": 8, "augmented_assign": 12, "unary": 2},
        )

    def test_published_schema_and_closed_nested_fields(self) -> None:
        schema = json.loads(SCHEMA.read_bytes())
        Draft202012Validator.check_schema(schema)
        validator = Draft202012Validator(schema)
        self.assertEqual(list(validator.iter_errors(self.document)), [])
        for key in ("opcodes", "decode", "unknown_lookups"):
            with self.subTest(table=key):
                changed = copy.deepcopy(self.document)
                changed[key][0]["extension"] = True
                self.assertTrue(list(validator.iter_errors(changed)))
                self.reject(changed)
        changed = copy.deepcopy(self.document)
        changed["operators"]["binary"][0]["extension"] = True
        self.assertTrue(list(validator.iter_errors(changed)))
        self.reject(changed)

    def test_root_profile_version_and_exact_field_set(self) -> None:
        for root in (None, [], "standalone-provider-v1", 1, True):
            self.reject(root)
        for value in (True, 1.0, "1", 0, 2, None):
            changed = copy.deepcopy(self.document)
            changed["schema_version"] = value
            self.reject(changed)
        for key in self.document:
            changed = copy.deepcopy(self.document)
            del changed[key]
            self.reject(changed)
        changed = copy.deepcopy(self.document)
        changed["unreviewed"] = None
        self.reject(changed)
        for value in ("standalone-provider-v2", "Standalone-provider-v1", None, True):
            changed = copy.deepcopy(self.document)
            changed["profile"] = value
            self.reject(changed)

    def test_every_decode_result_is_checked_against_literal_oracle(self) -> None:
        for byte in range(256):
            with self.subTest(byte=byte):
                changed = copy.deepcopy(self.document)
                row = changed["decode"][byte]
                row["symbol"] = "Halt" if row["symbol"] is None else None
                self.reject(changed)

    def test_every_opcode_and_operator_value_is_checked(self) -> None:
        for index in range(61):
            with self.subTest(opcode=index):
                changed = copy.deepcopy(self.document)
                changed["opcodes"][index]["byte"] ^= 1
                self.reject(changed)
        for group, rows in self.document["operators"].items():
            for index in range(len(rows)):
                with self.subTest(group=group, operator=index):
                    changed = copy.deepcopy(self.document)
                    changed["operators"][group][index]["symbol"] = "Halt"
                    self.reject(changed)

    def test_coordinated_self_consistent_symbol_swap_still_fails(self) -> None:
        # A round-trip-only checker would accept this bijection. Byte order,
        # field shapes and all assigned/hole counts remain unchanged.
        changed = copy.deepcopy(self.document)
        changed["opcodes"][0]["symbol"] = "Pop"
        changed["opcodes"][1]["symbol"] = "LoadConst"
        changed["decode"][1]["symbol"] = "Pop"
        changed["decode"][2]["symbol"] = "LoadConst"
        Draft202012Validator(json.loads(SCHEMA.read_bytes())).validate(changed)
        self.reject(changed)

    def test_coordinated_category_drift_and_high_nibble_holes_fail(self) -> None:
        changed = copy.deepcopy(self.document)
        changed["opcodes"][0]["category"] = "variable"
        changed["decode"][1]["category"] = "variable"
        self.reject(changed)
        for byte in (0, 7, 15, 22, 47, 57, 254):
            changed = copy.deepcopy(self.document)
            changed["decode"][byte]["category"] = "stack"
            self.reject(changed)

    def test_missing_extra_duplicate_reordered_and_nonlist_rows_fail(self) -> None:
        for field in ("opcodes", "decode", "unknown_lookups"):
            for action in ("missing", "extra", "duplicate", "reordered", "nonlist"):
                with self.subTest(field=field, action=action):
                    changed = copy.deepcopy(self.document)
                    rows = changed[field]
                    if action == "missing":
                        rows.pop()
                    elif action == "extra":
                        rows.append(rows[0])
                    elif action == "duplicate":
                        rows[1] = copy.deepcopy(rows[0])
                    elif action == "reordered":
                        rows[0], rows[1] = rows[1], rows[0]
                    else:
                        changed[field] = {}
                    self.reject(changed)
        changed = copy.deepcopy(self.document)
        changed["decode"] *= 3
        self.reject(changed)

    def test_byte_types_are_never_coerced_or_narrowed(self) -> None:
        for field in ("opcodes", "decode"):
            for value in (True, False, 1.0, -1, 256, 10**100, "1", None, [], {}):
                with self.subTest(field=field, value=value):
                    changed = copy.deepcopy(self.document)
                    changed[field][0]["byte"] = value
                    self.reject(changed)

    def test_labels_and_nested_objects_are_strict(self) -> None:
        for field in ("symbol", "category"):
            for value in (None, True, 1, [], {}, "", "x" * 65, "Load\ud800Const"):
                changed = copy.deepcopy(self.document)
                changed["opcodes"][0][field] = value
                self.reject(changed)
        for field in ("opcodes", "decode", "unknown_lookups"):
            changed = copy.deepcopy(self.document)
            changed[field][0] = []
            self.reject(changed)
        changed = copy.deepcopy(self.document)
        changed["decode"][1]["category"] = None
        self.reject(changed)

    def test_operator_groups_spelling_order_and_unknowns_are_exact(self) -> None:
        for group in self.document["operators"]:
            changed = copy.deepcopy(self.document)
            del changed["operators"][group]
            self.reject(changed)
            for rows in (None, [], self.document["operators"][group][::-1]):
                changed = copy.deepcopy(self.document)
                changed["operators"][group] = rows
                self.reject(changed)
        changed = copy.deepcopy(self.document)
        changed["operators"]["extra"] = []
        self.reject(changed)
        changed["operators"] = []
        self.reject(changed)
        for value in (None, 1, True, [], "x" * 33, "＋", " +", "+ "):
            changed = copy.deepcopy(self.document)
            changed["operators"]["binary"][0]["operator"] = value
            self.reject(changed)
        changed = copy.deepcopy(self.document)
        changed["operators"]["binary"][0] = None
        self.reject(changed)
        for index in range(8):
            for field, value in (
                ("group", "unary"),
                ("operator", "~"),
                ("symbol", "Not"),
            ):
                changed = copy.deepcopy(self.document)
                changed["unknown_lookups"][index][field] = value
                # Some probes already use unary; replace it by another group.
                if changed == self.document:
                    changed["unknown_lookups"][index][field] = "binary"
                self.reject(changed)

    def test_parse_rejects_ambiguous_numbers_keys_and_trailing_content(self) -> None:
        for raw in (
            b'{"a":1,"a":2}',
            b'{"a":1,"\\u0061":2}',
            b"NaN",
            b"Infinity",
            b"-Infinity",
            b"1e309",
            b"1.0",
            b"1e0",
            b"99999999999",
            b"0 0",
            b"{",
            b'"\\ud800"',
            b'{"\\udfff":0}',
            b"\xff",
            b"\xef\xbb\xbf{}",
        ):
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                MODULE.parse_json(raw)

    def test_transport_size_depth_escapes_and_scalar_boundaries(self) -> None:
        self.assertEqual(MODULE.parse_json(b"0" + b" " * 131071), 0)
        for raw in (b"0" + b" " * 131072, b"[" * 9 + b"0" + b"]" * 9):
            with self.assertRaises(ValueError):
                MODULE.parse_json(raw)
        self.assertEqual(
            MODULE.parse_json(b"[" * 8 + b"0" + b"]" * 8), [[[[[[[[0]]]]]]]]
        )
        literal = '{[\\"}]😀'
        self.assertEqual(MODULE.parse_json(json.dumps(literal).encode()), literal)
        with self.assertRaises(ValueError):
            MODULE.parse_json("{}")
        changed = copy.deepcopy(self.document)
        changed["extra"] = [[[[[[[[[0]]]]]]]]]
        self.reject(changed)
        changed = copy.deepcopy(self.document)
        changed["\ud800"] = 1
        self.reject(changed)

    def test_loader_bounds_reads_and_reports_missing_files(self) -> None:
        fake = io.BytesIO(b"x" * 131073)
        with (
            mock.patch.object(pathlib.Path, "open", return_value=fake),
            self.assertRaises(ValueError),
        ):
            MODULE.load_corpus(pathlib.Path("ignored"))
        with tempfile.TemporaryDirectory() as directory, self.assertRaises(OSError):
            MODULE.load_corpus(pathlib.Path(directory) / "absent.json")

    def test_cli_default_custom_invalid_missing_and_entrypoint(self) -> None:
        for args in ([], ["--fixture", str(CORPUS)]):
            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout):
                self.assertEqual(MODULE.main(args), 0)
            self.assertIn("359", stdout.getvalue())
            self.assertIn("native_adopters", stdout.getvalue())
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "invalid.json"
            path.write_bytes(b"{}")
            for candidate in (path, path.with_name("missing.json")):
                stderr = io.StringIO()
                with contextlib.redirect_stderr(stderr):
                    self.assertEqual(MODULE.main(["--fixture", str(candidate)]), 1)
                self.assertIn("STP01 invalid corpus", stderr.getvalue())
        with (
            mock.patch.object(sys, "argv", [str(SCRIPT)]),
            contextlib.redirect_stdout(io.StringIO()),
            self.assertRaises(SystemExit) as result,
        ):
            runpy.run_path(str(SCRIPT), run_name="__main__")
        self.assertEqual(result.exception.code, 0)


if __name__ == "__main__":
    unittest.main()
