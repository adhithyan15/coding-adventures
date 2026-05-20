"""Oracle tests for ``TIMEDIFF(A, B)`` — SQLite 3.43+ calendar diff.

``TIMEDIFF`` returns the difference between two time values as a
human-readable string of the form ``±YYYY-MM-DD HH:MM:SS.sss``.
Crucially, the year and month components use *calendar* arithmetic,
not simple seconds: ``timediff('2024-03-01', '2024-02-01')`` is one
month (``'+0000-01-00 …'``), regardless of February's day count.

The algorithm:
1. Parse both inputs.  If either parse fails or is NULL → NULL.
2. If A < B, swap and flip the sign so the magnitude is non-negative.
3. Walk the seven fields from microseconds up (µs, s, m, h, d, mo, y)
   borrowing one unit from the next-higher field whenever the current
   one goes negative.  The day borrow uses ``calendar.monthrange(…)``
   of the month preceding A's month — that's the calendar-aware part.
4. Truncate microseconds to milliseconds (3 decimal places).

Reference: https://sqlite.org/lang_datefunc.html#timediff
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(query: str) -> None:
    m = mini_sqlite.connect(":memory:").execute(query).fetchall()
    r = sqlite3.connect(":memory:").execute(query).fetchall()
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# Basic positive differences
# ---------------------------------------------------------------------------


class TestBasicPositive:
    def test_one_day_and_one_and_a_half_hours(self) -> None:
        _check(
            "SELECT timediff('2024-01-02 10:30:00', '2024-01-01 09:00:00')"
        )

    def test_zero_when_equal(self) -> None:
        _check("SELECT timediff('2024-01-01', '2024-01-01')")

    def test_fractional_seconds(self) -> None:
        _check(
            "SELECT timediff('2024-01-01 12:00:00.500', "
            "'2024-01-01 12:00:00.250')"
        )

    def test_millisecond_precision(self) -> None:
        _check(
            "SELECT timediff('2024-01-01 00:00:00.001', "
            "'2024-01-01 00:00:00.000')"
        )

    def test_iso8601_t_separator(self) -> None:
        _check(
            "SELECT timediff('2024-01-01T12:00:00', '2024-01-01T11:00:00')"
        )


# ---------------------------------------------------------------------------
# Negative differences — magnitude with leading minus sign
# ---------------------------------------------------------------------------


class TestNegative:
    def test_reverse_arguments(self) -> None:
        _check(
            "SELECT timediff('2024-01-01 09:00:00', '2024-01-02 10:30:00')"
        )

    def test_negative_one_year(self) -> None:
        _check("SELECT timediff('2023-01-01', '2024-01-01')")


# ---------------------------------------------------------------------------
# Year and month components — calendar arithmetic
# ---------------------------------------------------------------------------


class TestCalendarFields:
    def test_one_year_exactly(self) -> None:
        _check("SELECT timediff('2025-01-01', '2024-01-01')")

    def test_one_month_exactly(self) -> None:
        _check("SELECT timediff('2024-03-01', '2024-02-01')")

    def test_year_and_month(self) -> None:
        _check(
            "SELECT timediff('2025-02-15 10:00:00', '2024-01-10 11:30:00')"
        )

    def test_two_months_plus_days(self) -> None:
        _check("SELECT timediff('2024-03-15', '2024-01-10')")


# ---------------------------------------------------------------------------
# Day-of-month borrowing — the tricky case where Feb has only 28/29 days
# ---------------------------------------------------------------------------


class TestDayBorrow:
    def test_borrow_from_feb_leap(self) -> None:
        # 15 - 20 = -5, borrow 29 (Feb 2024 leap), gives 24 days.
        _check("SELECT timediff('2024-03-15', '2024-01-20')")

    def test_borrow_from_jan(self) -> None:
        # 29 - 31 = -2, borrow 31 (Jan), gives 29 days.
        _check("SELECT timediff('2024-02-29', '2024-01-31')")

    def test_hour_minute_day_chain_borrow(self) -> None:
        _check(
            "SELECT timediff('2024-01-31 12:00:00', '2024-01-01 13:30:00')"
        )

    def test_borrow_across_year_boundary(self) -> None:
        _check(
            "SELECT timediff('2024-01-01 00:00:00', '2023-12-31 23:00:00')"
        )


# ---------------------------------------------------------------------------
# NULL and invalid inputs
# ---------------------------------------------------------------------------


class TestNullAndInvalid:
    def test_null_first_argument(self) -> None:
        _check("SELECT timediff(NULL, '2024-01-01')")

    def test_null_second_argument(self) -> None:
        _check("SELECT timediff('2024-01-01 00:00:00', NULL)")

    def test_both_null(self) -> None:
        _check("SELECT timediff(NULL, NULL)")

    def test_invalid_first_argument(self) -> None:
        _check("SELECT timediff('not-a-date', '2024-01-01')")

    def test_invalid_second_argument(self) -> None:
        _check("SELECT timediff('2024-01-01', 'also-not-a-date')")


# ---------------------------------------------------------------------------
# Smoke test — 'now' arithmetic produces a non-NULL string
# ---------------------------------------------------------------------------


class TestNowSelf:
    def test_now_minus_now_is_zero(self) -> None:
        # 'now' resolves to the same instant in both arguments (we're
        # asking a single query), so the difference must be exactly
        # zero.  Both engines agree this is the all-zeros form.
        _check("SELECT timediff('now', 'now')")
