"""Oracle tests for ``date()/datetime()/time()`` with the ``'unixepoch'`` modifier.

SQLite's ``unixepoch`` modifier forces the time value to be read as
a Unix-epoch number — fundamentally different from the modifier's
SQLite docs framing as a post-processing step.  Concretely:

* Numeric time values (int or float) are read as seconds since
  1970-01-01 UTC.
* String time values are accepted **only** if the whole string is a
  valid number (optionally signed, optionally fractional, optionally
  whitespace-padded).  Strings that contain non-numeric characters
  — including ISO-8601 dates like ``'2024-01-15'`` — produce NULL,
  because SQLite refuses to interpret them as numeric Unix epochs.

Before this PR mini-sqlite ignored the modifier entirely: it would
still parse ``date('2024-01-15', 'unixepoch')`` as the ISO date
``'2024-01-15'``, and would reject pure numeric strings like
``date('1704067200', 'unixepoch')`` outright (because
``_parse_timevalue`` ran first and ISO-parsing failed).

The fix centralises ``unixepoch`` handling in ``_resolve_datetime``:
when the modifier appears in the chain, we coerce the time value to
a number ourselves (rejecting non-numeric strings via ``fullmatch``),
then strip the modifier from the chain so the downstream handler
doesn't re-process it.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(query: str) -> None:
    m = mini_sqlite.connect(":memory:").execute(query).fetchall()
    r = sqlite3.connect(":memory:").execute(query).fetchall()
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# unixepoch modifier rejects non-numeric strings
# ---------------------------------------------------------------------------


class TestUnixepochRejectsNonNumeric:
    def test_iso_date_string_rejected(self) -> None:
        # Was returning '2024-01-01' (modifier ignored); SQLite returns NULL.
        _check("SELECT date('2024-01-01', 'unixepoch')")

    def test_iso_datetime_string_rejected(self) -> None:
        _check("SELECT date('2024-01-01 00:00:00', 'unixepoch')")

    def test_partial_number_rejected(self) -> None:
        # Has '-' inside; SQLite treats whole string as non-numeric → NULL.
        _check("SELECT date('2024-01-15', 'unixepoch')")

    def test_number_with_trailing_garbage_rejected(self) -> None:
        # Different from CAST: unixepoch requires the *whole* string to
        # be numeric, not a prefix match.
        _check("SELECT date('2024abc', 'unixepoch')")

    def test_pure_garbage(self) -> None:
        _check("SELECT date('abc', 'unixepoch')")

    def test_null_input(self) -> None:
        _check("SELECT date(NULL, 'unixepoch')")


# ---------------------------------------------------------------------------
# unixepoch modifier accepts pure numeric strings
# ---------------------------------------------------------------------------


class TestUnixepochAcceptsNumeric:
    def test_int_string(self) -> None:
        # Was returning NULL (parse failed); SQLite returns '2024-01-01'.
        _check("SELECT date('1704067200', 'unixepoch')")

    def test_float_string(self) -> None:
        _check("SELECT date('1704067200.5', 'unixepoch')")

    def test_negative_string(self) -> None:
        # Pre-epoch dates work too.
        _check("SELECT date('-1234567890', 'unixepoch')")

    def test_explicit_plus_sign(self) -> None:
        _check("SELECT date('+1704067200', 'unixepoch')")

    def test_whitespace_padding(self) -> None:
        # SQLite tolerates surrounding whitespace.
        _check("SELECT date('  1704067200  ', 'unixepoch')")

    def test_raw_integer(self) -> None:
        # Already-numeric input continues to work.
        _check("SELECT date(1704067200, 'unixepoch')")

    def test_zero_is_epoch(self) -> None:
        _check("SELECT date(0, 'unixepoch')")


# ---------------------------------------------------------------------------
# datetime() and time() inherit the same rule
# ---------------------------------------------------------------------------


class TestUnixepochOtherFunctions:
    def test_datetime_numeric_string(self) -> None:
        _check("SELECT datetime('1704067200', 'unixepoch')")

    def test_datetime_iso_string_rejected(self) -> None:
        _check("SELECT datetime('2024-01-01', 'unixepoch')")

    def test_time_numeric_string(self) -> None:
        _check("SELECT time('1704067200', 'unixepoch')")

    def test_time_iso_string_rejected(self) -> None:
        _check("SELECT time('2024-01-01', 'unixepoch')")


# ---------------------------------------------------------------------------
# Regression: no modifier still works as before
# ---------------------------------------------------------------------------


class TestNoModifierRegression:
    def test_iso_date_string(self) -> None:
        _check("SELECT date('2024-01-01')")

    def test_iso_datetime_string(self) -> None:
        _check("SELECT date('2024-01-01 12:30:45')")

    def test_julian_day_float(self) -> None:
        _check("SELECT date(2460311.5)")
