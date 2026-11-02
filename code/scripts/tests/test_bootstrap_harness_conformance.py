"""Falsification tests for the TF01 self-corpus validator and TFV reference.

Every check the validator makes is shown to fire: each test corrupts the
corpus one way and expects a named problem. A validator that always said
"0 problems" would fail this file.
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
import tfv

SPEC = importlib.util.spec_from_file_location(
    "bootstrap_harness_conformance", SCRIPTS / "bootstrap_harness_conformance.py"
)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def _case(cases: list[dict], case_id: str) -> dict:
    return next(case for case in cases if case["id"] == case_id)


class ValidatorTests(unittest.TestCase):
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
        self.assertEqual(MODULE.check(self.cases), [])
        self.assertEqual(len(self.cases), 68)

    def test_flipped_verdict_is_caught(self) -> None:
        def flip(cases: list[dict]) -> None:
            case = _case(cases, "deep_equal/f64-negative-zero-is-not-zero")
            case["verdict"] = "pass"
            del case["failure_id"]

        self.assert_problem(flip, "reference says fail/not-deep-equal")

    def test_wrong_failure_id_is_caught(self) -> None:
        def swap(cases: list[dict]) -> None:
            _case(cases, "error_is/other-id")["failure_id"] = "no-error"

        self.assert_problem(swap, "reference says fail/wrong-error")

    def test_missing_negative_cases_are_caught(self) -> None:
        def only_passes(cases: list[dict]) -> None:
            cases[:] = [c for c in cases if c.get("verdict") != "fail"]

        problems = self.problems_after(only_passes)
        self.assertIn("deep_equal: needs at least one pass and one fail case", problems)
        self.assertTrue(any("failure ids never exercised" in p for p in problems))

    def test_unexercised_failure_id_is_caught(self) -> None:
        def drop(cases: list[dict]) -> None:
            cases[:] = [c for c in cases if c["id"] != "error_is/nothing-raised"]

        self.assert_problem(drop, "failure ids never exercised: ['no-error']")

    def test_missing_reference_vector_is_caught(self) -> None:
        def drop(cases: list[dict]) -> None:
            cases[:] = [c for c in cases if c["id"] != "splitmix64/seed-0"]

        self.assert_problem(drop, "seed-0 reference vector is missing")

    def test_wrong_primitive_outputs_are_caught(self) -> None:
        def corrupt(cases: list[dict]) -> None:
            _case(cases, "splitmix64/seed-1234567")["outputs"][1]["value"] = "1"
            _case(cases, "fnv1a64/a")["hash"]["value"] = "2"
            _case(cases, "shuffle/ten-seed-0")["order"].reverse()
            _case(cases, "sub_seed/a-seed-0")["sub_seed"]["value"] = "3"

        problems = self.problems_after(corrupt)
        for fragment in ("SplitMix64", "FNV-1a", "shuffle order", "sub-seed"):
            self.assertTrue(any(fragment in p for p in problems), fragment)

    def test_accepted_reject_text_is_caught(self) -> None:
        def make_valid(cases: list[dict]) -> None:
            _case(cases, "tfv_reject/uppercase-hex")["text"] = "tfv 1\nr R0\n"

        self.assert_problem(make_valid, "the strict parser accepted this text")

    def test_shape_problems_are_caught(self) -> None:
        def extra(cases: list[dict]) -> None:
            _case(cases, "fnv1a64/a")["unreviewed"] = True

        def unknown(cases: list[dict]) -> None:
            cases.append({"id": "x", "op": "teleport"})

        def duplicate(cases: list[dict]) -> None:
            cases.append(copy.deepcopy(cases[0]))

        self.assert_problem(extra, "fields")
        self.assert_problem(unknown, "unknown op 'teleport'")
        self.assert_problem(duplicate, "duplicate case id")


class CommandLineTests(unittest.TestCase):
    def run_main(self, fixture: pathlib.Path, *argv: str) -> tuple[int, str]:
        stderr = io.StringIO()
        with (
            mock.patch.object(MODULE, "FIXTURE", fixture),
            contextlib.redirect_stdout(io.StringIO()),
            contextlib.redirect_stderr(stderr),
        ):
            return MODULE.main(list(argv)), stderr.getvalue()

    def copy_fixture(self, into: pathlib.Path) -> pathlib.Path:
        for name in ("cases.json", "cases.tfv"):
            source = MODULE.FIXTURE / name
            (into / name).write_text(
                source.read_text(encoding="utf-8"), encoding="utf-8"
            )
        return into

    def test_real_fixture_passes(self) -> None:
        self.assertEqual(self.run_main(MODULE.FIXTURE)[0], 0)

    def test_stale_tfv_fails_and_write_repairs_it(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            fixture = self.copy_fixture(pathlib.Path(tmp))
            (fixture / "cases.tfv").write_text("tfv 1\n", encoding="utf-8")
            code, stderr = self.run_main(fixture)
            self.assertEqual(code, 1)
            self.assertIn("cases.tfv is stale", stderr)
            self.assertEqual(self.run_main(fixture, "--write")[0], 0)
            self.assertEqual(self.run_main(fixture)[0], 0)

    def test_duplicate_json_key_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            fixture = pathlib.Path(tmp)
            (fixture / "cases.json").write_text(
                '{"schema_version": 1, "schema_version": 1, "cases": []}',
                encoding="utf-8",
            )
            with self.assertRaises(SystemExit):
                self.run_main(fixture)

    def test_wrong_envelope_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            fixture = pathlib.Path(tmp)
            (fixture / "cases.json").write_text(
                json.dumps({"schema_version": 2, "cases": []}), encoding="utf-8"
            )
            with self.assertRaises(SystemExit):
                self.run_main(fixture)


class TfvTests(unittest.TestCase):
    def test_round_trip_of_every_kind(self) -> None:
        value = tfv.from_json(
            [
                None,
                True,
                False,
                "hé",
                {"kind": "int", "value": "-18446744073709551617"},
                {"kind": "f64", "bits": "8000000000000000"},
                {"kind": "bytes", "hex": ""},
                {"kind": "map", "entries": [["k", []]]},
            ]
        )
        text = tfv.encode_file([("r", [("v", value)])])
        self.assertEqual(tfv.parse_file(text), [("r", [("v", value)])])

    def test_json_shorthands_are_strict(self) -> None:
        for bad in (
            1,
            1.5,
            {"kind": "int", "value": 5},
            {"kind": "int", "value": "-0"},
            {"kind": "int", "value": "007"},
            {"kind": "f64", "bits": "3FF0000000000000"},
            {"kind": "bytes", "hex": "abc"},
            {"kind": "map", "entries": [["a"]]},
            {"kind": "map", "entries": [["a", 1], ["a", 2]]},
            {"kind": "set", "items": []},
            {"kind": "int", "value": "1", "extra": 0},
        ):
            with self.assertRaises(tfv.TfvError, msg=repr(bad)):
                tfv.from_json(bad)

    def test_nesting_limit_matches_the_rust_lane(self) -> None:
        def nest(depth: int) -> object:
            node: object = None
            for _ in range(depth):
                node = [node]
            return node

        tfv.from_json(nest(tfv.MAX_DEPTH))
        with self.assertRaises(tfv.TfvError):
            tfv.from_json(nest(tfv.MAX_DEPTH + 1))
        tfv.parse_file("tfv 1\nr R1 Kv " + "L1 " * tfv.MAX_DEPTH + "N\n")
        with self.assertRaises(tfv.TfvError):
            tfv.parse_file("tfv 1\nr R1 Kv " + "L1 " * (tfv.MAX_DEPTH + 1) + "N\n")

    def test_encoder_refuses_bad_names(self) -> None:
        with self.assertRaises(tfv.TfvError):
            tfv.encode_record("bad id", [])
        with self.assertRaises(tfv.TfvError):
            tfv.encode_record("r", [("Bad", ("null",))])
        with self.assertRaises(tfv.TfvError):
            tfv.encode_file([("r", []), ("r", [])])

    def test_equality_rules(self) -> None:
        nan_a, nan_b = ("f64", 0x7FF8000000000000), ("f64", 0x7FF0000000000001)
        self.assertTrue(tfv.values_equal(nan_a, nan_b))
        self.assertFalse(tfv.values_equal(("f64", 0), ("f64", 1 << 63)))
        self.assertFalse(tfv.values_equal(("int", 1), ("f64", 0x3FF0000000000000)))
        left = ("map", [(("text", "a"), ("int", 1)), (("text", "b"), ("int", 2))])
        right = ("map", [(("text", "b"), ("int", 2)), (("text", "a"), ("int", 1))])
        self.assertTrue(tfv.values_equal(left, right))
        self.assertFalse(tfv.values_equal(left, ("map", left[1][:1])))


if __name__ == "__main__":
    unittest.main()
