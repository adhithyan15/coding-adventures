"""Oracle tests for ``date(..., '+N year[s]')`` Feb 29 rollover.

SQLite's ``date(timevalue, '+N year')`` follows the same overflow rule
as its ``+N month`` counterpart: if the resulting (year, month, day)
isn't a real date, it rolls forward into the next month rather than
clamping the day.

Concretely::

    date('2024-02-29', '+1 year')

The naive answer is ``2025-02-29``, which doesn't exist (2025 is not
a leap year).  Mini-sqlite previously clamped to ``2025-02-28``.
SQLite rolls over: the date is one day past Feb 28, so the answer is
``2025-03-01``.

The fix mirrors the existing month-rollover algorithm:

* Try the literal ``(year + n, month, day)``.
* On ``ValueError`` (e.g. Feb 29 in a non-leap year), compute the
  overflow ``day - last_valid_day`` and add it as extra days starting
  from the month's last valid day.

The month-arithmetic path (``'+1 month'``) was already correct because
that's where the algorithm originally lived; this PR ports the same
logic to the year branch.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(query: str) -> None:
    m = mini_sqlite.connect(":memory:").execute(query).fetchall()
    r = sqlite3.connect(":memory:").execute(query).fetchall()
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# Feb 29 → next year: must roll over to Mar 1
# ---------------------------------------------------------------------------


class TestLeapDayForwardRollover:
    def test_one_year(self) -> None:
        # Was returning '2025-02-28'; SQLite returns '2025-03-01'.
        _check("SELECT date('2024-02-29', '+1 year')")

    def test_two_years(self) -> None:
        # 2026 is also non-leap.
        _check("SELECT date('2024-02-29', '+2 years')")

    def test_three_years(self) -> None:
        # 2027 non-leap.
        _check("SELECT date('2024-02-29', '+3 years')")

    def test_five_years(self) -> None:
        # 2029 non-leap.
        _check("SELECT date('2024-02-29', '+5 years')")


class TestLeapDayBackwardRollover:
    def test_minus_one_year(self) -> None:
        # 2023 is non-leap; SQLite returns '2023-03-01'.
        _check("SELECT date('2024-02-29', '-1 year')")

    def test_minus_three_years(self) -> None:
        # 2021 non-leap.
        _check("SELECT date('2024-02-29', '-3 years')")


class TestLeapDayToLeapYear:
    def test_plus_four_years(self) -> None:
        # 2028 is also a leap year — date is preserved.
        _check("SELECT date('2024-02-29', '+4 years')")

    def test_minus_four_years(self) -> None:
        _check("SELECT date('2024-02-29', '-4 years')")

    def test_plus_eight_years(self) -> None:
        # Across multiple leap cycles.
        _check("SELECT date('2024-02-29', '+8 years')")


# ---------------------------------------------------------------------------
# Regression: ordinary (non-Feb-29) dates still work
# ---------------------------------------------------------------------------


class TestRegularYearArithmetic:
    def test_january_15(self) -> None:
        _check("SELECT date('2024-01-15', '+1 year')")

    def test_june_15(self) -> None:
        _check("SELECT date('2024-06-15', '+10 years')")

    def test_december_31(self) -> None:
        # Dec 31 in any year exists; no rollover needed.
        _check("SELECT date('2024-12-31', '+1 year')")

    def test_january_1(self) -> None:
        _check("SELECT date('2024-01-01', '+100 years')")

    def test_year_with_time_component(self) -> None:
        # Time component must be preserved.
        _check("SELECT datetime('2024-02-29 12:34:56', '+1 year')")
