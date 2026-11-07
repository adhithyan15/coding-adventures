"""Falsification tests for the VER05 PBT conformance validator and reference.

Every check the validator makes is shown to fire: each test corrupts the
corpus one way and asserts that a named problem is reported. A validator that
always returned zero problems would fail this file.
"""

from __future__ import annotations

import contextlib
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
import tfv  # noqa: E402

SPEC = importlib.util.spec_from_file_location(
    "pbt_conformance", SCRIPTS / "pbt_conformance.py"
)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def _case(cases: list[dict], case_id: str) -> dict:
    return next(case for case in cases if case["id"] == case_id)


class PbtValidatorTests(unittest.TestCase):
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

    def test_corrupted_primitive_bool_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            _case(cases, "pbt/gen_primitive/bool_seed0")["outputs"][0] = False

        self.assert_problem(corrupt, "gen_primitive outputs mismatch")

    def test_corrupted_primitive_u8_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            _case(cases, "pbt/gen_primitive/u8_seed0")["outputs"][0]["value"] = "999"

        self.assert_problem(corrupt, "gen_primitive outputs mismatch")

    def test_corrupted_primitive_u64_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            _case(cases, "pbt/gen_primitive/u64_seed0")["outputs"][0]["value"] = "0"

        self.assert_problem(corrupt, "gen_primitive outputs mismatch")

    def test_corrupted_primitive_i128_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            _case(cases, "pbt/gen_primitive/i128_seed42")["outputs"][0]["value"] = "1"

        self.assert_problem(corrupt, "gen_primitive outputs mismatch")

    def test_corrupted_bounded_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            _case(cases, "pbt/gen_bounded/small_seed7")["outputs"][0]["value"] = "99"

        self.assert_problem(corrupt, "gen_bounded outputs mismatch")

    def test_corrupted_bytes_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            _case(cases, "pbt/gen_bytes/len4_8_seed13")["outputs"][0]["hex"] = (
                "ffffffff"
            )

        self.assert_problem(corrupt, "gen_bytes outputs mismatch")

    def test_corrupted_shrink_int_minimal_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            _case(cases, "pbt/shrink_int/54321_le_100")["expected_minimal"]["value"] = (
                "100"
            )

        self.assert_problem(corrupt, "shrink_int minimal mismatch")

    def test_corrupted_shrink_int_steps_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            _case(cases, "pbt/shrink_int/54321_le_100")["expected_steps"]["value"] = "1"

        self.assert_problem(corrupt, "shrink_int step count mismatch")

    def test_corrupted_shrink_refinement_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            _case(cases, "pbt/shrink_refinement/x_le_50_min10")["expected_minimal"][
                "value"
            ] = "50"

        self.assert_problem(corrupt, "shrink_refinement minimal mismatch")

    def test_corrupted_shrink_list_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            _case(cases, "pbt/shrink_list/consecutive_duplicates")[
                "expected_minimal"
            ] = [
                {"kind": "int", "value": "1"},
                {"kind": "int", "value": "1"},
            ]

        self.assert_problem(corrupt, "shrink_list minimal mismatch")

    def test_corrupted_shrink_list_steps_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            _case(cases, "pbt/shrink_list/consecutive_duplicates")["expected_steps"][
                "value"
            ] = "99"

        self.assert_problem(corrupt, "shrink_list steps mismatch")

    def test_corrupted_shrink_bytes_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            _case(cases, "pbt/shrink_bytes/len_lt_5")["expected_minimal"]["hex"] = (
                "0000"
            )

        self.assert_problem(corrupt, "shrink_bytes minimal mismatch")

    def test_corrupted_shrink_bytes_steps_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            _case(cases, "pbt/shrink_bytes/len_lt_5")["expected_steps"]["value"] = "99"

        self.assert_problem(corrupt, "shrink_bytes steps mismatch")

    def test_corrupted_replay_initial_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            _case(cases, "pbt/replay/seed42_le_100")["initial_fail"]["value"] = "999"

        self.assert_problem(corrupt, "replay initial_fail mismatch")

    def test_corrupted_replay_minimal_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            _case(cases, "pbt/replay/seed42_le_100")["expected_minimal"]["value"] = (
                "999"
            )

        self.assert_problem(corrupt, "replay minimal mismatch")

    def test_corrupted_filter_exhausted_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            _case(cases, "pbt/filter_exhausted/impossible")["verdict"] = "pass"

        self.assert_problem(
            corrupt, "filter_exhausted must report fail/property-exhausted"
        )

    def test_corrupted_failure_id_valid_is_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            _case(cases, "pbt/failure_id/counterexample")["valid"] = False

        self.assert_problem(corrupt, "failure_id_valid membership disagrees")

    def test_accepted_corpus_reject_is_caught(self) -> None:
        def make_valid(cases: list[dict]) -> None:
            _case(cases, "pbt/corpus_reject/bad_header")["text"] = "tfv 1\nr R0\n"

        self.assert_problem(
            make_valid, "corpus_reject text was accepted by strict parser"
        )

    def test_duplicate_case_id_is_caught(self) -> None:
        def dup(cases: list[dict]) -> None:
            cases.append(copy.deepcopy(cases[0]))

        self.assert_problem(dup, "duplicate case id")

    def test_invalid_case_id_type_is_caught(self) -> None:
        def bad_id(cases: list[dict]) -> None:
            cases.append({"id": 123, "op": "gen_primitive"})

        self.assert_problem(bad_id, "case missing valid string id")

    def test_unknown_operation_is_caught(self) -> None:
        def unknown(cases: list[dict]) -> None:
            cases.append({"id": "pbt/unknown", "op": "magic_draw"})

        self.assert_problem(unknown, "unknown op 'magic_draw'")

    def test_shape_mismatch_is_caught(self) -> None:
        def bad_shape(cases: list[dict]) -> None:
            _case(cases, "pbt/gen_primitive/bool_seed0")["extra_field"] = "bad"

        self.assert_problem(bad_shape, "fields")

    def test_missing_operation_coverage_is_caught(self) -> None:
        def drop_op(cases: list[dict]) -> None:
            cases[:] = [c for c in cases if c["op"] != "gen_bytes"]

        self.assert_problem(drop_op, "operations never exercised in corpus")

    def test_missing_registered_failure_id_is_caught(self) -> None:
        def drop_id(cases: list[dict]) -> None:
            cases[:] = [c for c in cases if c["id"] != "pbt/failure_id/panic"]

        self.assert_problem(drop_id, "registered failure IDs missing")

    def test_missing_unregistered_id_rejection_is_caught(self) -> None:
        def drop_unreg(cases: list[dict]) -> None:
            cases[:] = [c for c in cases if c["id"] != "pbt/failure_id/unregistered"]

        self.assert_problem(
            drop_unreg, "failure_id_valid needs at least one unregistered-ID rejection"
        )

    def test_tfv_conversion_error_is_caught(self) -> None:
        def bad_tfv(cases: list[dict]) -> None:
            _case(cases, "pbt/gen_primitive/bool_seed0")["outputs"] = 42

        self.assert_problem(bad_tfv, "TFV conversion error")

    def test_sub_seed_primitives(self) -> None:
        s1 = MODULE.sub_seed(0, "prop1")
        s2 = MODULE.sub_seed(0, "prop2")
        self.assertNotEqual(s1, s2)
        # Deterministic
        self.assertEqual(s1, MODULE.sub_seed(0, "prop1"))

    def test_gen_primitive_unknown_type(self) -> None:
        with self.assertRaises(ValueError):
            MODULE.gen_primitive(0, "float128", 1)

    def test_shrink_int_direct_target(self) -> None:
        # If target itself falsifies property, it should be chosen immediately
        MODULE.PROPERTIES["int_ge_10"] = lambda x: x >= 10
        min_w, steps = MODULE.shrink_int(5, 0, 0, 20, "int_ge_10")
        self.assertEqual(min_w, 0)
        self.assertEqual(steps, 1)

    def test_shrink_list_empty_falsifies(self) -> None:
        # If empty list falsifies, it should be returned in 1 step
        MODULE.PROPERTIES["list_sum_gt_100"] = lambda lst: sum(lst) > 100
        min_w, steps = MODULE.shrink_list([5, 10], 0, "list_sum_gt_100")
        self.assertEqual(min_w, [])
        self.assertEqual(steps, 1)

    def test_shrink_list_right_half(self) -> None:
        # A list property where only the right half falsifies
        MODULE.PROPERTIES["has_no_gt_10"] = lambda lst: not any(x > 10 for x in lst)
        min_w, steps = MODULE.shrink_list([1, 2, 15, 20], 0, "has_no_gt_10")
        self.assertTrue(any(x > 10 for x in min_w))

    def test_schema_version_check(self) -> None:
        with tempfile_named(json.dumps({"schema_version": 2, "cases": []})) as p:
            with mock.patch.object(MODULE, "FIXTURE", p.parent):
                with self.assertRaises(SystemExit):
                    MODULE.load_cases()

    def test_duplicate_json_keys(self) -> None:
        with self.assertRaises(SystemExit):
            MODULE._no_duplicate_keys([("a", 1), ("a", 2)])

    def test_cli_main_check(self) -> None:
        stdout = io.StringIO()
        with mock.patch("sys.stdout", stdout):
            code = MODULE.main([])
        self.assertEqual(code, 0)
        self.assertIn("25 cases", stdout.getvalue())

    def test_cli_main_write(self) -> None:
        stdout = io.StringIO()
        with mock.patch("sys.stdout", stdout):
            code = MODULE.main(["--write"])
        self.assertEqual(code, 0)
        self.assertIn("25 cases", stdout.getvalue())

    def test_cli_main_failure_output(self) -> None:
        stderr = io.StringIO()
        with mock.patch.object(MODULE, "check", return_value=["synthetic failure"]):
            with mock.patch("sys.stderr", stderr):
                code = MODULE.main([])
        self.assertEqual(code, 1)
        self.assertIn("synthetic failure", stderr.getvalue())

    def test_cli_main_missing_tfv(self) -> None:
        stderr = io.StringIO()
        with tempfile.TemporaryDirectory() as td:
            t_dir = pathlib.Path(td)
            (t_dir / "cases.json").write_text(
                (MODULE.FIXTURE / "cases.json").read_text(encoding="utf-8"),
                encoding="utf-8",
            )
            with mock.patch.object(MODULE, "FIXTURE", t_dir):
                with mock.patch("sys.stderr", stderr):
                    code = MODULE.main([])
        self.assertEqual(code, 1)
        self.assertIn("cases.tfv is stale", stderr.getvalue())

    def test_cli_main_stale_tfv(self) -> None:
        stderr = io.StringIO()
        with tempfile.TemporaryDirectory() as td:
            t_dir = pathlib.Path(td)
            (t_dir / "cases.json").write_text(
                (MODULE.FIXTURE / "cases.json").read_text(encoding="utf-8"),
                encoding="utf-8",
            )
            (t_dir / "cases.tfv").write_text("stale content", encoding="utf-8")
            with mock.patch.object(MODULE, "FIXTURE", t_dir):
                with mock.patch("sys.stderr", stderr):
                    code = MODULE.main([])
        self.assertEqual(code, 1)
        self.assertIn("cases.tfv is stale", stderr.getvalue())

    def test_round_trip_cases_tfv(self) -> None:
        records = MODULE.to_records(self.cases)
        encoded = tfv.encode_file(records)
        parsed = tfv.parse_file(encoded)
        self.assertEqual(parsed, records)


@contextlib.contextmanager
def tempfile_named(content: str):
    with tempfile.TemporaryDirectory() as td:
        p = pathlib.Path(td) / "cases.json"
        p.write_text(content, encoding="utf-8")
        yield p


if __name__ == "__main__":
    unittest.main()
