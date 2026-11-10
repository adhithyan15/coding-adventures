"""Falsify tfv_differential.py: it must pass agreement and fail every disagreement."""

from __future__ import annotations

import pathlib
import sys
import tempfile
import unittest

SCRIPTS = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import tfv
import tfv_differential as differential


def hex_of(text: str) -> str:
    return text.encode("utf-8").hex()


def digest_of(text: str) -> int:
    ok, digest = differential.python_verdict(text)
    assert ok and digest is not None
    return digest


class DifferentialCheckerTests(unittest.TestCase):
    def check(self, content: str) -> list[str]:
        with tempfile.NamedTemporaryFile("w", suffix=".tsv", delete=False) as handle:
            handle.write(content)
        try:
            _, _, problems = differential.check_documents(handle.name)
        finally:
            pathlib.Path(handle.name).unlink()
        return problems

    def test_agreement_on_accept_and_reject_passes(self) -> None:
        good = "tfv 1\nr R1 Kv I5\n"
        bad = "tfv 1\nr R1 Kv I05\n"
        content = (
            f"DOC\t{hex_of(good)}\tok\t{digest_of(good):016x}\n"
            f"DOC\t{hex_of(bad)}\terr\t-\n"
        )
        self.assertEqual(self.check(content), [])

    def test_rust_rejecting_what_python_accepts_is_reported(self) -> None:
        good = "tfv 1\nr R0\n"
        self.assertEqual(len(self.check(f"DOC\t{hex_of(good)}\terr\t-\n")), 1)

    def test_rust_accepting_what_python_rejects_is_reported(self) -> None:
        bad = "tfv 1\nr R1 Kv I-0\n"
        self.assertEqual(len(self.check(f"DOC\t{hex_of(bad)}\tok\t{0:016x}\n")), 1)

    def test_different_canonical_forms_are_reported(self) -> None:
        good = "tfv 1\nr R1 Kv S61\n"
        wrong = digest_of(good) ^ 0xFF
        self.assertEqual(len(self.check(f"DOC\t{hex_of(good)}\tok\t{wrong:016x}\n")), 1)

    def test_a_wrong_matrix_cell_is_reported(self) -> None:
        with tempfile.NamedTemporaryFile("w", suffix=".matrix", delete=False) as handle:
            handle.write(f"UNI\t{hex_of('N')}\nUNI\t{hex_of('T')}\nROW\t10\nROW\t11\n")
        try:
            cells, problems = differential.check_matrix(handle.name)
        finally:
            pathlib.Path(handle.name).unlink()
        self.assertEqual(cells, 4)
        # Row 1 should read "01" (T differs from N, equals itself); "11" claims
        # T equals N, which is the single wrong cell, (1, 0).
        self.assertEqual(len(problems), 1)

    def test_nan_payloads_compare_equal_in_the_reference(self) -> None:
        a = differential.parse_universe_token("D7ff8000000000000")
        b = differential.parse_universe_token("D7ff0000000000001")
        self.assertTrue(tfv.values_equal(a, b))

    def test_builtin_self_test_passes(self) -> None:
        self.assertEqual(differential.self_test(), 0)


if __name__ == "__main__":
    unittest.main()
