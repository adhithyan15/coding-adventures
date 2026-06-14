"""Oracle tests for SQLite-compatible CAST string-to-number coercion.

Companion to ``test_cast_numeric_prefix.py`` in ``sql-vm``.  Each test
runs the same ``SELECT CAST(...)`` query against both mini-sqlite and
the reference ``sqlite3`` module and asserts byte-for-byte equality.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(query: str) -> None:
    m = mini_sqlite.connect(":memory:").execute(query).fetchall()
    r = sqlite3.connect(":memory:").execute(query).fetchall()
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


class TestCastRealOracle:
    def test_inf_keyword_rejected(self) -> None:
        # Was returning Python's float('inf'); SQLite returns 0.0.
        _check("SELECT CAST('inf' AS REAL)")

    def test_nan_keyword_rejected(self) -> None:
        _check("SELECT CAST('nan' AS REAL)")

    def test_infinity_keyword_rejected(self) -> None:
        _check("SELECT CAST('infinity' AS REAL)")

    def test_number_with_trailing_garbage(self) -> None:
        # Was returning 0.0 (Python rejects the whole string); SQLite
        # extracts the float prefix.
        _check("SELECT CAST('1.5abc' AS REAL)")

    def test_overflow_to_inf_unchanged(self) -> None:
        # SQLite produces inf here too, so this is a regression guard.
        _check("SELECT CAST('1e500' AS REAL)")

    def test_decimal_only(self) -> None:
        _check("SELECT CAST('.5' AS REAL)")

    def test_leading_sign(self) -> None:
        _check("SELECT CAST('+1.5' AS REAL)")
        _check("SELECT CAST('-1.5' AS REAL)")


class TestCastIntegerOracle:
    def test_number_with_trailing_garbage(self) -> None:
        # Was returning 0; SQLite extracts the integer prefix.
        _check("SELECT CAST('123abc' AS INTEGER)")

    def test_negative_with_trailing_garbage(self) -> None:
        _check("SELECT CAST('-42abc' AS INTEGER)")

    def test_float_string_takes_int_prefix(self) -> None:
        # SQLite's INTEGER cast stops at the decimal point: result is 1.
        _check("SELECT CAST('1.5abc' AS INTEGER)")
        _check("SELECT CAST('1.9' AS INTEGER)")
        _check("SELECT CAST('-1.9' AS INTEGER)")

    def test_exponent_marker_stops_parsing(self) -> None:
        _check("SELECT CAST('1e5' AS INTEGER)")
        _check("SELECT CAST('1e5abc' AS INTEGER)")

    def test_pure_garbage(self) -> None:
        _check("SELECT CAST('abc' AS INTEGER)")
        _check("SELECT CAST('' AS INTEGER)")
