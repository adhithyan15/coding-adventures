"""Oracle tests for two ``strftime()`` bugs.

1. ``%f`` (SQLite's millisecond format ``SS.sss``) was always emitting
   ``.000`` for inputs that included a fractional-seconds suffix.  The
   ISO-8601 datetime parser in ``_parse_timevalue`` had a fast path
   that truncated the input via slice arithmetic to fit the bare
   ``%Y-%m-%d %H:%M:%S`` strptime format — the fraction was silently
   discarded *before* the dedicated fractional-seconds branch could
   try to capture it.  Fix: try the fractional-seconds regex first.

2. ``%W`` (week of year, Monday-based, 00–53) was off by one.  The
   custom implementation used ``isocalendar()[1] - 1``, which produces
   ISO-week numbering shifted — different from POSIX week-of-year.
   Python's ``strftime('%W')`` already produces SQLite-compatible
   output, so the fix is to remove ``%W`` from the manual substitution
   list and let Python's strftime handle it directly.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(query: str) -> None:
    m = mini_sqlite.connect(":memory:").execute(query).fetchall()
    r = sqlite3.connect(":memory:").execute(query).fetchall()
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# %f — fractional seconds preserved through _parse_timevalue
# ---------------------------------------------------------------------------


class TestFractionalSeconds:
    def test_full_datetime_with_milliseconds(self) -> None:
        # Was returning '45.000'; now correctly '45.123'.
        _check("SELECT strftime('%f', '2024-01-15 12:30:45.123')")

    def test_full_datetime_isoT_with_milliseconds(self) -> None:
        # T-separator variant.
        _check("SELECT strftime('%f', '2024-01-15T12:30:45.123')")

    def test_time_only_with_milliseconds(self) -> None:
        # Time-only inputs were already correct; regression guard.
        _check("SELECT strftime('%f', '12:30:45.123')")

    def test_microseconds_truncated_to_milliseconds(self) -> None:
        # SQLite outputs only 3 decimal places — the extra precision is
        # dropped, not rounded.
        _check("SELECT strftime('%f', '2024-01-15 12:30:45.000123')")

    def test_compose_full_iso8601_with_fraction(self) -> None:
        _check(
            "SELECT strftime('%Y-%m-%dT%H:%M:%fZ', '2024-01-15 12:30:45.123')"
        )

    def test_partial_fraction(self) -> None:
        # Single-digit fraction should pad correctly.
        _check("SELECT strftime('%f', '2024-01-15 12:30:45.1')")


# ---------------------------------------------------------------------------
# %W — week of year (Monday-based, 00–53)
# ---------------------------------------------------------------------------


class TestWeekOfYear:
    def test_week_01(self) -> None:
        # Was returning '00' (off by one).
        _check("SELECT strftime('%W', '2024-01-01')")

    def test_week_02(self) -> None:
        _check("SELECT strftime('%W', '2024-01-08')")

    def test_week_03(self) -> None:
        # The original probe that surfaced the bug.
        _check("SELECT strftime('%W', '2024-01-15')")

    def test_week_53(self) -> None:
        # Last week of a 366-day leap year.
        _check("SELECT strftime('%W', '2024-12-31')")

    def test_week_00(self) -> None:
        # Year starting on a Tuesday/.../Sunday has a partial week 00.
        _check("SELECT strftime('%W', '2023-01-01')")


# ---------------------------------------------------------------------------
# Regression — other strftime specifiers still work
# ---------------------------------------------------------------------------


class TestStrftimeRegression:
    def test_year_month_day(self) -> None:
        _check("SELECT strftime('%Y-%m-%d', '2024-01-15')")

    def test_unix_epoch(self) -> None:
        _check("SELECT strftime('%s', '2024-01-01')")

    def test_julian_day(self) -> None:
        _check("SELECT strftime('%J', '2024-01-01')")

    def test_day_of_year(self) -> None:
        _check("SELECT strftime('%j', '2024-12-31')")
