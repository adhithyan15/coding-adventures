"""Falsification tests for the VER06 fuzz engine conformance validator and reference.

Every check the validator makes is shown to fire: each test corrupts the
corpus one way and asserts that a named problem is reported. A validator that
always returned zero problems would fail this file.
"""

from __future__ import annotations

import copy
import importlib.util
import io
import json
import pathlib
import sys
import tempfile
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
SCRIPTS = ROOT / "code" / "scripts"
sys.path.insert(0, str(SCRIPTS))
import tfv

SPEC = importlib.util.spec_from_file_location(
    "fuzz_conformance", SCRIPTS / "fuzz_conformance.py"
)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def _case(cases: list[dict], case_id: str) -> dict:
    return next(case for case in cases if case["id"] == case_id)


class FuzzValidatorTests(unittest.TestCase):
    def setUp(self) -> None:
        self.cases = MODULE.load_cases()

    def problems_after(self, mutate) -> list[str]:
        cases = copy.deepcopy(self.cases)
        mutate(cases)
        return MODULE.check(cases)

    def assert_problem(self, mutate, fragment: str) -> None:
        problems = self.problems_after(mutate)
        self.assertTrue(
            any(fragment in problem for problem in problems),
            f"expected a problem mentioning {fragment!r}, got {problems}",
        )

    def test_canonical_corpus_is_clean(self) -> None:
        problems = MODULE.check(self.cases)
        self.assertEqual(problems, [])
        self.assertEqual(len(self.cases), 25)

    def test_corrupted_bit_flip_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            c = _case(cases, "fuzz/mutate_bit_flip/bit0")
            c["output"]["hex"] = "ffff"

        self.assert_problem(corrupt, "bit_flip expected")

    def test_corrupted_byte_flip_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            c = _case(cases, "fuzz/mutate_byte_flip/byte0")
            c["output"]["hex"] = "000000"

        self.assert_problem(corrupt, "byte_flip expected")

    def test_corrupted_arithmetic_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            c = _case(cases, "fuzz/mutate_arithmetic/u8_add1")
            c["output"]["hex"] = "9900"

        self.assert_problem(corrupt, "byte_arithmetic expected")

    def test_corrupted_boundary_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            c = _case(cases, "fuzz/mutate_boundary/u8_max")
            c["output"]["hex"] = "0000"

        self.assert_problem(corrupt, "boundary_insert expected")

    def test_corrupted_block_delete_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            c = _case(cases, "fuzz/mutate_block_delete/middle")
            c["output"]["hex"] = "0102030405"

        self.assert_problem(corrupt, "block_delete expected")

    def test_corrupted_block_insert_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            c = _case(cases, "fuzz/mutate_block_insert/middle")
            c["output"]["hex"] = "000000"

        self.assert_problem(corrupt, "block_insert expected")

    def test_corrupted_block_duplicate_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            c = _case(cases, "fuzz/mutate_block_duplicate/prefix")
            c["output"]["hex"] = "0000"

        self.assert_problem(corrupt, "block_duplicate expected")

    def test_corrupted_splice_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            c = _case(cases, "fuzz/mutate_splice/cross")
            c["output"]["hex"] = "00000000"

        self.assert_problem(corrupt, "splice expected")

    def test_corrupted_sequence_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            c = _case(cases, "fuzz/mutate_sequence/seed42_steps4")
            c["output"]["hex"] = "0000"

        self.assert_problem(corrupt, "mutate_sequence expected")

    def test_corrupted_corpus_admit_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            c = _case(cases, "fuzz/corpus_admit/novel_edge")
            c["admitted"] = False

        self.assert_problem(corrupt, "corpus_admit expected")

    def test_corrupted_crash_dedup_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            c = _case(cases, "fuzz/crash_dedup/same_signature")
            c["is_duplicate"] = False

        self.assert_problem(corrupt, "crash_dedup expected")

    def test_corrupted_failure_id_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            c = _case(cases, "fuzz/failure_id/crash_valid")
            c["valid"] = False

        self.assert_problem(corrupt, "failure_id_valid expected")

    def test_privacy_leak_in_reporting_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            c = _case(cases, "fuzz/private_reporting/opaque_token")
            # Leak the raw payload into the tap line
            c["tap_line"] = "not ok 1 - ct_compare [fuzz-crash:3a8f19b2] payload:deadbeef"

        self.assert_problem(corrupt, "tap_line mismatch")

    def test_duplicate_case_id_rejected(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            cases.append(copy.deepcopy(cases[0]))

        self.assert_problem(corrupt, "duplicate case id")

    def test_missing_case_id_rejected(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            del cases[0]["id"]

        self.assert_problem(corrupt, "missing or non-string id")

    def test_unknown_op_rejected(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            cases[0]["op"] = "unsupported_op"

        self.assert_problem(corrupt, "unknown op")

    def test_missing_field_rejected(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            del cases[0]["input"]

        self.assert_problem(corrupt, "fields")

    def test_missing_operation_in_corpus_rejected(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            # Remove all cases with op 'mutate_splice'
            cases[:] = [c for c in cases if c["op"] != "mutate_splice"]

        self.assert_problem(corrupt, "corpus missing operations")

    def test_corpus_reject_accept_rejected(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            c = _case(cases, "fuzz/corpus_reject/bad_header")
            # Valid TFV header that strict parser accepts
            c["text"] = "tfv 1\ncase R1 Kop S6f70\n"

        self.assert_problem(corrupt, "expected corpus_reject text to fail decode")

    def test_sub_seed_generation(self) -> None:
        s1 = MODULE.sub_seed(0, "target_a")
        s2 = MODULE.sub_seed(0, "target_b")
        s3 = MODULE.sub_seed(0, "target_a")
        self.assertNotEqual(s1, s2)
        self.assertEqual(s1, s3)

    def test_empty_mutator_inputs(self) -> None:
        self.assertEqual(MODULE.bit_flip(b"", 0, 1), b"")
        self.assertEqual(MODULE.byte_flip(b"", 0, 1), b"")
        self.assertEqual(MODULE.byte_arithmetic(b"", 0, 1, 1), b"")
        self.assertEqual(MODULE.boundary_insert(b"", 0, 1, 255), b"")
        self.assertEqual(MODULE.block_delete(b"", 0, 1), b"")
        self.assertEqual(MODULE.block_insert(b"", 0, b"abc"), b"abc")
        self.assertEqual(MODULE.block_duplicate(b"", 0, 1, 0), b"")

    def test_mutate_sequence_coverage(self) -> None:
        # Exercises all 7 mutation operations and empty buffer handling
        out1 = MODULE.mutate_sequence(b"1234567890abcdef", 12345, 50)
        self.assertTrue(len(out1) > 0)
        out2 = MODULE.mutate_sequence(b"", 99, 10)
        self.assertTrue(len(out2) > 0)

    def test_to_records_and_roundtrip(self) -> None:
        records = MODULE.to_records(self.cases)
        self.assertEqual(len(records), len(self.cases))
        encoded = tfv.encode_file(records)
        decoded = tfv.parse_file(encoded)
        self.assertEqual(len(decoded), len(records))

    def test_cli_main_success(self) -> None:
        code = MODULE.main([])
        self.assertEqual(code, 0)

    def test_cli_main_failure_output(self) -> None:
        stderr = io.StringIO()
        with (
            mock.patch.object(MODULE, "check", return_value=["synthetic failure"]),
            mock.patch("sys.stderr", stderr),
        ):
            code = MODULE.main([])
        self.assertEqual(code, 1)
        self.assertIn("synthetic failure", stderr.getvalue())

    def test_cli_main_write_flag(self) -> None:
        with tempfile.TemporaryDirectory() as tmp_dir:
            t_dir = pathlib.Path(tmp_dir)
            (t_dir / "cases.json").write_text(
                json.dumps({"schema_version": 1, "cases": self.cases}),
                encoding="utf-8",
            )
            with mock.patch.object(MODULE, "FIXTURE", t_dir):
                code = MODULE.main(["--write"])
            self.assertEqual(code, 0)
            self.assertTrue((t_dir / "cases.tfv").exists())

    def test_cli_main_stale_tfv(self) -> None:
        with tempfile.TemporaryDirectory() as tmp_dir:
            t_dir = pathlib.Path(tmp_dir)
            (t_dir / "cases.json").write_text(
                json.dumps({"schema_version": 1, "cases": self.cases}),
                encoding="utf-8",
            )
            (t_dir / "cases.tfv").write_text("stale content\n", encoding="utf-8")
            stderr = io.StringIO()
            with (
                mock.patch.object(MODULE, "FIXTURE", t_dir),
                mock.patch("sys.stderr", stderr),
            ):
                code = MODULE.main([])
            self.assertEqual(code, 1)
            self.assertIn("stale", stderr.getvalue())

    def test_schema_version_check(self) -> None:
        with tempfile.TemporaryDirectory() as tmp_dir:
            t_dir = pathlib.Path(tmp_dir)
            (t_dir / "cases.json").write_text(
                json.dumps({"schema_version": 2, "cases": []}),
                encoding="utf-8",
            )
            with (
                mock.patch.object(MODULE, "FIXTURE", t_dir),
                self.assertRaises(SystemExit),
            ):
                MODULE.load_cases()


if __name__ == "__main__":
    unittest.main()
