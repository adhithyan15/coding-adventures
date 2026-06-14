"""Oracle tests for ``ROUND(x[, n])`` — match SQLite byte-for-byte.

SQLite breaks ties on ``round`` by rounding *half away from zero* —
the school-arithmetic convention.  Mini-sqlite was delegating to
Python's built-in ``round`` which uses *banker's rounding* (round half
to even); the difference shows up on every even half-integer::

    Python:  round(0.5) == 0    round(2.5) == 2
    SQLite:  round(0.5) == 1.0  round(2.5) == 3.0

This file pairs each interesting input against the reference
``sqlite3`` module and asserts byte-for-byte equality.  See the
companion ``test_round_half_away_from_zero.py`` in the ``sql-vm``
package for finer-grained unit tests against the scalar function
registry directly.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(query: str) -> None:
    m = mini_sqlite.connect(":memory:").execute(query).fetchall()
    r = sqlite3.connect(":memory:").execute(query).fetchall()
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# One-argument form — half-away-from-zero ties
# ---------------------------------------------------------------------------


class TestRoundOneArgOracle:
    def test_half_positive(self) -> None:
        _check("SELECT round(0.5), round(1.5), round(2.5), round(3.5), round(4.5)")

    def test_half_negative(self) -> None:
        _check("SELECT round(-0.5), round(-1.5), round(-2.5), round(-3.5)")

    def test_below_and_above_half(self) -> None:
        _check("SELECT round(1.4), round(1.6), round(-1.4), round(-1.6)")


# ---------------------------------------------------------------------------
# Two-argument form — precision-aware rounding on IEEE 754 value
# ---------------------------------------------------------------------------


class TestRoundTwoArgOracle:
    def test_simple_decimal_places(self) -> None:
        _check("SELECT round(3.14159, 2), round(3.14159, 4)")

    def test_tie_cases_decimal(self) -> None:
        # 0.25 is exactly representable; rounds half-up at the tenths place.
        _check("SELECT round(0.25, 1), round(1.25, 1), round(2.5, 0)")

    def test_apparent_ties_that_are_not(self) -> None:
        # These look like ties when written as decimal literals but the
        # IEEE 754 representation is slightly below — so they round down.
        _check("SELECT round(2.355, 2), round(1.005, 2), round(0.15, 1)")

    def test_negative_digits_clamped(self) -> None:
        # SQLite clamps n < 0 to 0 — no rounding to the left of the decimal.
        _check("SELECT round(1234.5, -1), round(1250.0, -2)")


# ---------------------------------------------------------------------------
# NULL handling
# ---------------------------------------------------------------------------


class TestRoundNullOracle:
    def test_null_value(self) -> None:
        _check("SELECT round(NULL), round(NULL, 2)")

    def test_null_digits(self) -> None:
        # Regression — mini-sqlite previously coerced NULL → 0 and returned
        # the rounded value.
        _check("SELECT round(1.5, NULL), round(2.5, NULL)")
