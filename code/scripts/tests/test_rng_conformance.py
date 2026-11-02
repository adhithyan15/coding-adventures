"""Falsification tests for the rng corpus validator.

Each test corrupts the corpus one way and expects a named problem, so a
validator that always reported "0 problems" would fail here.
"""

from __future__ import annotations

import contextlib
import copy
import importlib.util
import io
import json
import pathlib
import tempfile
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
SPEC = importlib.util.spec_from_file_location(
    "rng_conformance", ROOT / "code" / "scripts" / "rng_conformance.py"
)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def _case(document: dict, case_id: str) -> dict:
    return next(case for case in document["cases"] if case["id"] == case_id)


class RngCorpusTests(unittest.TestCase):
    def setUp(self) -> None:
        self.document = MODULE.load()

    def problems_after(self, mutate) -> list[str]:
        document = copy.deepcopy(self.document)
        mutate(document)
        return MODULE.check(document)

    def assert_problem(self, mutate, fragment: str) -> None:
        problems = self.problems_after(mutate)
        self.assertTrue(
            any(fragment in problem for problem in problems),
            f"expected a problem mentioning {fragment!r}, got {problems}",
        )

    def test_canonical_corpus_is_clean(self) -> None:
        self.assertEqual(MODULE.check(self.document), [])
        self.assertEqual(len(self.document["cases"]), 27)

    def test_wrong_result_is_caught(self) -> None:
        def corrupt(document: dict) -> None:
            call = _case(document, "pcg32/range-64-bit-regime")["calls"][0]
            call["result"] = str(int(call["result"]) + 1)

        self.assert_problem(corrupt, "results differ from the reference")

    def test_wrong_float_bits_are_caught(self) -> None:
        def corrupt(document: dict) -> None:
            _case(document, "lcg/seed-1-basics")["calls"][4]["result_bits"] = "0" * 16

        self.assert_problem(corrupt, "results differ from the reference")

    def test_missing_regimes_are_caught(self) -> None:
        def drop(document: dict) -> None:
            document["cases"] = [
                case for case in document["cases"] if "rejects" not in case["id"]
            ]

        problems = self.problems_after(drop)
        for generator in ("lcg", "xorshift64", "pcg32"):
            self.assertTrue(
                any(p.startswith(generator) and "reject" in p for p in problems),
                f"{generator}: {problems}",
            )

    def test_missing_whole_domain_and_error_cases_are_caught(self) -> None:
        def drop(document: dict) -> None:
            document["cases"] = [
                case
                for case in document["cases"]
                if "whole" not in case["id"] and "error" not in case["id"]
            ]

        problems = self.problems_after(drop)
        self.assertTrue(any("'error'" in p and "'full'" in p for p in problems))

    def test_shape_problems_are_caught(self) -> None:
        def extra_field(document: dict) -> None:
            document["cases"][0]["note"] = "x"

        def bad_generator(document: dict) -> None:
            document["cases"][0]["generator"] = "mt19937"

        def bad_seed(document: dict) -> None:
            document["cases"][0]["seed"] = "007"

        def bad_bound(document: dict) -> None:
            _case(document, "lcg/range-32-bit-regime")["calls"][0]["min"] = "+1"

        def huge_bound(document: dict) -> None:
            _case(document, "lcg/range-32-bit-regime")["calls"][0]["min"] = "9" * 5000

        def duplicate(document: dict) -> None:
            document["cases"].append(copy.deepcopy(document["cases"][0]))

        self.assert_problem(extra_field, "fields must be")
        self.assert_problem(bad_generator, "unknown generator")
        self.assert_problem(bad_seed, "seed must be")
        self.assert_problem(bad_bound, "non-canonical integer")
        self.assert_problem(huge_bound, "non-canonical integer")
        self.assert_problem(duplicate, "duplicate case id")

    def test_out_of_domain_bounds_are_errors_not_results(self) -> None:
        draws = MODULE.Draws("lcg", 1)
        for bounds in ((1, 0), (MODULE.I64_MIN - 1, 0), (0, MODULE.I64_MAX + 1)):
            with self.assertRaises(ValueError):
                draws.next_int_in_range(*bounds)

    def test_reference_matches_the_published_table(self) -> None:
        for generator, values in MODULE.PUBLISHED_SEED_1.items():
            draws = MODULE.Draws(generator, 1)
            self.assertEqual([draws.next_u32() for _ in values], values)


class CommandLineTests(unittest.TestCase):
    def run_main(self, fixture: pathlib.Path, *argv: str) -> tuple[int, str]:
        stderr = io.StringIO()
        with (
            mock.patch.object(MODULE, "FIXTURE", fixture),
            contextlib.redirect_stdout(io.StringIO()),
            contextlib.redirect_stderr(stderr),
        ):
            return MODULE.main(list(argv)), stderr.getvalue()

    def test_real_fixture_passes(self) -> None:
        self.assertEqual(self.run_main(MODULE.FIXTURE)[0], 0)

    def test_write_fills_in_results_for_calls_without_them(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            fixture = pathlib.Path(tmp) / "cases.json"
            document = MODULE.load()
            for case in document["cases"]:
                case["calls"] = [
                    {key: call[key] for key in ("op", "min", "max") if key in call}
                    for call in case["calls"]
                ]
            fixture.write_text(json.dumps(document), encoding="utf-8")
            self.assertEqual(self.run_main(fixture)[0], 1)
            self.assertEqual(self.run_main(fixture, "--write")[0], 0)
            self.assertEqual(json.loads(fixture.read_text()), MODULE.load())

    def test_duplicate_json_key_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            fixture = pathlib.Path(tmp) / "cases.json"
            fixture.write_text('{"schema_version": 1, "schema_version": 1}')
            with self.assertRaises(SystemExit):
                self.run_main(fixture)


if __name__ == "__main__":
    unittest.main()
