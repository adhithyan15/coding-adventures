"""Oracle tests for ``printf('%.Nf', x)`` half-away-from-zero rounding.

Python's ``'%f'`` formatting uses banker's rounding (round half to
even), which produces ``printf('%.0f', 4.5) == '4'``.  SQLite (and
C's printf in general) round half away from zero — same convention as
the school-arithmetic ``round()`` scalar function — so the result is
``'5'``.

PR #3668 fixed ``round()`` by pre-quantizing through
``Decimal.quantize`` with ``ROUND_HALF_UP``.  This PR applies the
same approach to ``printf`` whenever the conversion is ``f`` (or
``F``) and a precision is specified.  Other conversions (``%e``,
``%g``) use significant-digit rounding, which already produces
matching output across both engines for every test we care about, so
they continue to delegate to Python's formatter.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(query: str) -> None:
    m = mini_sqlite.connect(":memory:").execute(query).fetchall()
    r = sqlite3.connect(":memory:").execute(query).fetchall()
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# %.0f at integer half points
# ---------------------------------------------------------------------------


class TestPrecisionZero:
    def test_half_positive_integers(self) -> None:
        # 0.5 → 1, 2.5 → 3, 4.5 → 5 — all rounded up (away from zero).
        _check("SELECT printf('%.0f', 0.5)")
        _check("SELECT printf('%.0f', 2.5)")
        _check("SELECT printf('%.0f', 4.5)")

    def test_half_negative_integers(self) -> None:
        # -0.5 → -1, -2.5 → -3 — sign retained, magnitude rounds up.
        _check("SELECT printf('%.0f', -0.5)")
        _check("SELECT printf('%.0f', -2.5)")

    def test_non_half_unchanged(self) -> None:
        _check("SELECT printf('%.0f', 3.7)")
        _check("SELECT printf('%.0f', 3.2)")


# ---------------------------------------------------------------------------
# Non-zero precisions
# ---------------------------------------------------------------------------


class TestPrecisionN:
    def test_one_decimal_place_half(self) -> None:
        _check("SELECT printf('%.1f', 0.25)")
        _check("SELECT printf('%.1f', 1.25)")
        _check("SELECT printf('%.1f', 1.35)")

    def test_three_decimal_places(self) -> None:
        _check("SELECT printf('%.3f', 1.2345)")

    def test_ieee754_representation(self) -> None:
        # 2.355 is actually 2.3549999... in float64, so SQLite rounds
        # down even though "2.355 rounded" would naively be "2.36".
        # Mini-sqlite must agree with SQLite, not the naive answer.
        _check("SELECT printf('%.2f', 2.355)")


# ---------------------------------------------------------------------------
# Other conversions are unaffected
# ---------------------------------------------------------------------------


class TestOtherConversions:
    def test_g_unchanged(self) -> None:
        _check("SELECT printf('%g', 0.0001)")

    def test_e_unchanged(self) -> None:
        _check("SELECT printf('%e', 1.5)")

    def test_integer_unchanged(self) -> None:
        _check("SELECT printf('%d', 42)")
