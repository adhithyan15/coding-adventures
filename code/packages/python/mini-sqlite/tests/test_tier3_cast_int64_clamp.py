"""Oracle tests for INTEGER cast saturation at the signed-64-bit endpoints.

SQLite's INTEGER affinity is a signed 64-bit value.  ``CAST(...)`` to
INTEGER **saturates** at ``2**63 - 1`` (= 9_223_372_036_854_775_807)
and ``-2**63`` (= -9_223_372_036_854_775_808) rather than wrapping or
preserving arbitrary-precision Python ints.

Before this PR mini-sqlite let the bigint flow through unclamped,
producing values that real sqlite3 would never return — a subtle
type-affinity mismatch.  All four numeric→int paths (bool, float,
str-prefix, native int) now clamp through ``_clamp_int64``.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(query: str) -> None:
    m = mini_sqlite.connect(":memory:").execute(query).fetchall()
    r = sqlite3.connect(":memory:").execute(query).fetchall()
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# Numeric literal at or near int64 boundary
# ---------------------------------------------------------------------------


class TestNumericLiteral:
    def test_int64_max_exact(self) -> None:
        _check("SELECT CAST(9223372036854775807 AS INTEGER)")

    def test_int64_max_plus_one(self) -> None:
        _check("SELECT CAST(9223372036854775808 AS INTEGER)")

    def test_far_above_int64_max(self) -> None:
        _check("SELECT CAST(99999999999999999999 AS INTEGER)")

    def test_int64_min_exact(self) -> None:
        _check("SELECT CAST(-9223372036854775808 AS INTEGER)")

    def test_int64_min_minus_one(self) -> None:
        _check("SELECT CAST(-9223372036854775809 AS INTEGER)")

    def test_far_below_int64_min(self) -> None:
        _check("SELECT CAST(-99999999999999999999 AS INTEGER)")


# ---------------------------------------------------------------------------
# String forms of out-of-range integers
# ---------------------------------------------------------------------------


class TestStringOutOfRange:
    def test_string_above_int64_max(self) -> None:
        _check("SELECT CAST('99999999999999999999' AS INTEGER)")

    def test_string_below_int64_min(self) -> None:
        _check("SELECT CAST('-99999999999999999999' AS INTEGER)")

    def test_string_at_int64_max(self) -> None:
        _check("SELECT CAST('9223372036854775807' AS INTEGER)")

    def test_string_at_int64_min(self) -> None:
        _check("SELECT CAST('-9223372036854775808' AS INTEGER)")


# ---------------------------------------------------------------------------
# Regression — normal-range values still work
# ---------------------------------------------------------------------------


class TestNormalRange:
    def test_small_positive(self) -> None:
        _check("SELECT CAST(42 AS INTEGER)")

    def test_small_negative(self) -> None:
        _check("SELECT CAST(-42 AS INTEGER)")

    def test_zero(self) -> None:
        _check("SELECT CAST(0 AS INTEGER)")

    def test_float_truncation_in_range(self) -> None:
        _check("SELECT CAST(1.5 AS INTEGER)")

    def test_string_in_range(self) -> None:
        _check("SELECT CAST('42' AS INTEGER)")

    def test_string_prefix_garbage(self) -> None:
        # Regression for PR #3699 — INTEGER cast extracts the digit
        # prefix and clamps the result.
        _check("SELECT CAST('123abc' AS INTEGER)")
