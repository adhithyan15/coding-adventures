"""Adversarial BF03 profile tests, independent of all native IR implementations."""

from __future__ import annotations

import contextlib
import copy
import importlib.util
import io
import json
import runpy
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "code/scripts/compiler_ir_conformance.py"
CORPUS = ROOT / "code/specs/fixtures/compiler-ir-v1/cases.json"
SPEC = importlib.util.spec_from_file_location("bf03", SCRIPT)
assert SPEC and SPEC.loader
M = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(M)

# Hand-derived from the governing spec, not a call to either printer or fixture.
GOLDEN = (
    ".version 1\n\n.entry _start\n\n_start:\n"
    "  LOAD_IMM   v0, -1  ; #42\n  ; note\n  NOP  ; #7\n  HALT  ; #99\n"
)


class CoreProfileTests(unittest.TestCase):
    def setUp(self):
        self.doc = json.loads(CORPUS.read_bytes())
        self.program = copy.deepcopy(self.doc["programs"][2]["program"])

    def test_complete_corpus_and_literal_golden(self):
        result = M.validate_document(self.doc)
        self.assertEqual(
            result,
            {
                "profile": "core-text-v1",
                "opcodes": 25,
                "cases": 50,
                "native_adopters_checked": 0,
            },
        )
        self.assertEqual(M.load_corpus(CORPUS), self.doc)
        self.assertEqual(self.doc["programs"][2]["text"], GOLDEN)
        self.assertEqual(M.print_program(self.program), GOLDEN)
        self.assertEqual(M.parse_text(GOLDEN), self.program)
        self.assertEqual([r["name"] for r in self.doc["opcodes"]][15], "LABEL")
        for row in self.doc["programs"]:
            self.assertEqual(M.parse_text(row["text"]), row["program"])
            self.assertEqual(M.print_program(row["program"]), row["text"])
        self.assertEqual(M.id_sequence(2147483647, 1), ([2147483647], 2147483648))
        first, _ = M.id_sequence(0, 3)
        first[0] = 99
        self.assertEqual(M.id_sequence(0, 3), ([0, 1, 2], 3))

    def test_schema_structure_closed_fields(self):
        schema = json.loads(CORPUS.with_name("schema.json").read_bytes())
        Draft202012Validator.check_schema(schema)
        validator = Draft202012Validator(schema)
        validator.validate(self.doc)
        changed = copy.deepcopy(self.doc)
        changed["programs"][0]["extension"] = 1
        self.assertTrue(list(validator.iter_errors(changed)))
        with self.assertRaises(ValueError):
            M.validate_document(changed)

    def test_every_negative_case_is_actually_rejected(self):
        for key, field, function in (
            ("invalid_programs", "program", M.validate_program),
            ("invalid_texts", "text", M.parse_text),
        ):
            for row in self.doc[key]:
                with self.subTest(name=row["name"]), self.assertRaises(ValueError):
                    function(row[field])

    def test_profile_roster_oracle_and_coordinated_mutations(self):
        mutations = [
            lambda d: d.update(profile="core-text-v2"),
            lambda d: d.update(schema_version=True),
            lambda d: d["opcodes"].reverse(),
            lambda d: d["opcodes"][0].update(category="memory"),
            lambda d: d["opcodes"][0].update(operands=["immediate", "register"]),
            lambda d: d["programs"].pop(),
            lambda d: d["invalid_texts"].pop(),
            lambda d: d["id_sequences"][0].update(values=[1, 2, 3], next=4),
            lambda d: d["invalid_programs"][0].update(
                program=d["programs"][0]["program"]
            ),
            lambda d: d["invalid_texts"][0].update(text=d["programs"][0]["text"]),
            lambda d: d["programs"][2].update(text=GOLDEN.replace("#42", "#43")),
        ]
        for mutate in mutations:
            changed = copy.deepcopy(self.doc)
            mutate(changed)
            with self.assertRaises(ValueError):
                M.validate_document(changed)
        # Mutually consistent ID/text corruption still fails the literal witness.
        changed = copy.deepcopy(self.doc)
        changed["programs"][2]["program"]["instructions"][1]["id"] = 43
        changed["programs"][2]["text"] = GOLDEN.replace("#42", "#43")
        with self.assertRaises(ValueError):
            M.validate_document(changed)
        for key in self.doc:
            changed = copy.deepcopy(self.doc)
            del changed[key]
            with self.assertRaises(ValueError):
                M.validate_document(changed)
        for key in ("programs", "invalid_programs", "invalid_texts", "id_sequences"):
            changed = copy.deepcopy(self.doc)
            changed[key][0]["name"] = changed[key][1]["name"]
            with self.assertRaises(ValueError):
                M.validate_document(changed)

    def test_typed_domains_missing_extra_types_and_ownership(self):
        for value in (None, [], True, 1):
            with self.assertRaises(ValueError):
                M.validate_program(value)
        for key in self.program:
            changed = copy.deepcopy(self.program)
            del changed[key]
            with self.assertRaises(ValueError):
                M.validate_program(changed)
        for field, values in (
            ("version", [True, 1.0, "1", 0]),
            ("entry", [None, "", "v0", "é", "x" * 65, "a:b"]),
            ("instructions", [None]),
            ("data", [None]),
        ):
            for value in values:
                changed = copy.deepcopy(self.program)
                changed[field] = value
                with self.assertRaises(ValueError):
                    M.validate_program(changed)
        for value in (True, 1.0, "1", -1, 2147483648):
            changed = copy.deepcopy(self.program)
            changed["instructions"][1]["id"] = value
            with self.assertRaises(ValueError):
                M.validate_program(changed)
        for operand in (
            None,
            {},
            {"kind": "unknown"},
            {"kind": "register", "index": -1},
            {"kind": "register", "index": True},
            {"kind": "immediate", "value": -2147483649},
        ):
            changed = copy.deepcopy(self.program)
            changed["instructions"][1]["operands"][0] = operand
            with self.assertRaises(ValueError):
                M.validate_program(changed)
        for text in (
            "",
            " note",
            "note ",
            "#note",
            "a;b",
            "a:b",
            "a\nb",
            "é",
            "x" * 257,
        ):
            changed = copy.deepcopy(self.program)
            changed["instructions"][2]["operands"][0]["value"] = text
            with self.assertRaises(ValueError):
                M.validate_program(changed)
        for field, value in (
            ("size", -1),
            ("size", 65536),
            ("init", 256),
            ("init", True),
        ):
            changed = copy.deepcopy(self.program)
            changed["data"] = [{"label": "x", "size": 0, "init": 0}]
            changed["data"][0][field] = value
            with self.assertRaises(ValueError):
                M.validate_program(changed)
        before = copy.deepcopy(self.program)
        M.print_program(self.program)
        a = M.parse_text(GOLDEN)
        a["instructions"][1]["operands"][0]["index"] = 55
        self.assertEqual(M.parse_text(GOLDEN), before)
        self.assertEqual(self.program, before)

    def test_named_negative_witnesses_cannot_degrade_to_generic_rejections(self):
        for key, field, replacement in (
            ("invalid_programs", "program", None),
            ("invalid_texts", "text", ""),
            ("invalid_texts", "text", None),
        ):
            for index in range(len(self.doc[key])):
                changed = copy.deepcopy(self.doc)
                changed[key][index][field] = replacement
                if changed == self.doc:
                    continue
                with self.subTest(key=key, index=index), self.assertRaises(ValueError):
                    M.validate_document(changed)

    def test_positive_and_generator_purposes_cannot_be_erased(self):
        for index in range(4):
            changed = copy.deepcopy(self.doc)
            source = self.doc["programs"][(index + 1) % 4]
            changed["programs"][index].update(
                program=source["program"], text=source["text"]
            )
            with self.assertRaises(ValueError):
                M.validate_document(changed)
        changed = copy.deepcopy(self.doc)
        changed["id_sequences"][0].update(start=1, count=0, values=[], next=1)
        with self.assertRaises(ValueError):
            M.validate_document(changed)
        changed = copy.deepcopy(self.doc)
        changed["invalid_programs"][0]["program"]["version"] = True
        with self.assertRaises(ValueError):
            M.validate_document(changed)

    def test_exact_resource_boundaries_and_order(self):
        p = {"version": 1, "entry": "_start", "data": [], "instructions": []}
        header = ".version 1\n\n.entry _start\n"
        width, extra = divmod(65536 - len(header), 256)
        p["instructions"] = [
            {
                "op": "COMMENT",
                "id": -1,
                "operands": [
                    {"kind": "text", "value": "x" * (width - 5 + (index < extra))}
                ],
            }
            for index in range(256)
        ]
        text = M.print_program(p)
        self.assertEqual(len(text.encode()), 65536)
        self.assertEqual(M.parse_text(text), p)
        with self.assertRaises(ValueError):
            M.parse_text(text + "\n")
        p["instructions"][0]["operands"][0]["value"] += "x"
        with self.assertRaises(ValueError):
            M.print_program(p)
        p["instructions"] = []
        p["data"] = [
            {"label": f"d{i}", "size": 65535, "init": 255} for i in reversed(range(32))
        ]
        self.assertEqual(M.parse_text(M.print_program(p)), p)
        for source in (
            ".version 1\n\n.data x 1\n\n.entry _start\n",
            ".version 1\n" + "\n.data x 1 0\n" * 33 + "\n.entry _start\n",
            header + "\n",
            header + ("  NOP  ; #0\n" * 257),
            header + "  LOAD_IMM   v01, 0  ; #0\n",
            header + "  LOAD_IMM   v0, 99999999999999999999999  ; #0\n",
        ):
            with self.assertRaises(ValueError):
                M.parse_text(source)
        # Aggregate rejection precedes roster/semantic comparisons. Individually
        # small programs cannot bypass the shared corpus work budget.
        for mode in ("instructions", "data"):
            changed = copy.deepcopy(self.doc)
            for row in changed["invalid_programs"]:
                row["program"][mode] = (
                    [{"op": "NOP", "id": i, "operands": []} for i in range(256)]
                    if mode == "instructions"
                    else [{"label": f"d{i}", "size": 0, "init": 0} for i in range(32)]
                )
            with self.assertRaisesRegex(ValueError, "aggregate"):
                M.validate_document(changed)

    def test_strict_transport_numbers_keys_unicode_and_bounds(self):
        for raw in (
            b'{"a":1,"\\u0061":2}',
            b'{"a":{"b":0,"b":1}}',
            b'"\\ud800"',
            b'"\\udfff"',
            b'"\\ud800x"',
            b"NaN",
            b"Infinity",
            b"-Infinity",
            b"1.0",
            b"1e0",
            b"99999999999",
            b"{} {}",
            b"\xff",
            b"\xef\xbb\xbf{}",
            b"{",
        ):
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                M.parse_json(raw)
        self.assertEqual(M.parse_json(b'"\\ud83d\\ude00"'), "😀")
        self.assertEqual(M.parse_json(b"0" + b" " * (M.MAX_BYTES - 1)), 0)
        self.assertEqual(
            M.parse_json(b"[" * 16 + b"0" + b"]" * 16),
            [[[[[[[[[[[[[[[[0]]]]]]]]]]]]]]]],
        )
        for raw in (b"0" + b" " * M.MAX_BYTES, b"[" * 17 + b"0" + b"]" * 17, "{}"):
            with self.assertRaises(ValueError):
                M.parse_json(raw)
        quoted = '{[\\"}]'
        self.assertEqual(M.parse_json(json.dumps(quoted).encode()), quoted)
        cycle = []
        cycle.append(cycle)
        for value in (
            cycle,
            {"x": "\ud800"},
            {"x": "a" * (M.MAX_BYTES + 1)},
            [0] * 100001,
        ):
            with self.assertRaises(ValueError):
                M.validate_document(value)

    def test_resource_caps_and_generator_atomicity(self):
        for start, count in (
            (-1, 0),
            (True, 1),
            (0, True),
            (0, -1),
            (0, 257),
            (2147483648, 0),
            (2147483647, 2),
        ):
            with self.assertRaises(ValueError):
                M.id_sequence(start, count)
        self.assertEqual(len(M.id_sequence(0, 256)[0]), 256)
        for key, count in (("instructions", 257), ("data", 33)):
            p = copy.deepcopy(self.program)
            p[key] = [
                p["instructions"][0]
                if key == "instructions"
                else {"label": "x", "size": 0, "init": 0}
            ] * count
            with self.assertRaises(ValueError):
                M.validate_program(p)
        with self.assertRaises(ValueError):
            M.parse_text("x" * 65537)
        for value in (None, b"text", "\ud800", "\x00"):
            with self.assertRaises(ValueError):
                M.parse_text(value)
        p = {"version": 1, "entry": "unresolved", "data": [], "instructions": []}
        self.assertEqual(M.parse_text(M.print_program(p)), p)
        p["instructions"] = [
            {
                "op": "COMMENT",
                "id": -1,
                "operands": [{"kind": "text", "value": "x" * 256}],
            }
        ] * 256
        with self.assertRaises(ValueError):
            M.print_program(p)
        changed = copy.deepcopy(self.doc)
        changed["programs"] *= 33
        with self.assertRaises(ValueError):
            M.validate_document(changed)

    def test_cli_read_bound_and_entrypoint(self):
        for args in ([], ["--fixture", str(CORPUS)]):
            with contextlib.redirect_stdout(io.StringIO()) as out:
                self.assertEqual(M.main(args), 0)
            self.assertEqual(json.loads(out.getvalue())["native_adopters_checked"], 0)
        with tempfile.TemporaryDirectory() as directory:
            invalid = Path(directory) / "invalid.json"
            invalid.write_bytes(b"{}")
            for path in (invalid, invalid.with_name("missing.json")):
                with contextlib.redirect_stderr(io.StringIO()):
                    self.assertEqual(M.main(["--fixture", str(path)]), 1)
        with (
            mock.patch.object(
                Path, "open", return_value=io.BytesIO(b"x" * (M.MAX_BYTES + 1))
            ),
            self.assertRaises(ValueError),
        ):
            M.load_corpus(Path("ignored"))
        with (
            mock.patch.object(sys, "argv", [str(SCRIPT)]),
            contextlib.redirect_stdout(io.StringIO()),
            self.assertRaises(SystemExit) as result,
        ):
            runpy.run_path(str(SCRIPT), run_name="__main__")
        self.assertEqual(result.exception.code, 0)


if __name__ == "__main__":
    unittest.main()
