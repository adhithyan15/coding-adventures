"""Tests for ``CURRENT_DATE`` / ``CURRENT_TIME`` / ``CURRENT_TIMESTAMP``.

SQLite recognises these three identifiers (case-insensitive) as
*expressions* — not column references and not function calls — that
return the current UTC date/time::

    CURRENT_DATE      ⟶  'YYYY-MM-DD'                (10 chars)
    CURRENT_TIME      ⟶  'HH:MM:SS'                  (8  chars)
    CURRENT_TIMESTAMP ⟶  'YYYY-MM-DD HH:MM:SS'       (19 chars)

Mini-sqlite previously failed with ``unknown column: 'CURRENT_DATE'``
because the lexer emits these as bare NAME tokens (the SQL token
grammar doesn't list them as keywords).  The adapter now intercepts
single-name ``column_ref`` nodes whose name matches one of the three
and rewrites them to the equivalent scalar-function call
(``date('now')`` / ``time('now')`` / ``datetime('now')``) — both
already implemented in the VM.

The exact wallclock value naturally differs each run, so these tests
focus on **shape** (length, format) rather than equality with sqlite3.
A few cross-checks against sqlite3 confirm format compatibility, and
the keyword path is verified in expression position (SELECT list,
WHERE clause, CASE branch).
"""

from __future__ import annotations

import re
import sqlite3

import mini_sqlite


def _connect() -> mini_sqlite.Connection:
    return mini_sqlite.connect(":memory:")


class TestShape:
    """Lengths and format strings match SQLite's documented spec."""

    def test_current_date_length(self) -> None:
        assert _connect().execute("SELECT length(CURRENT_DATE)").fetchall() == [(10,)]

    def test_current_time_length(self) -> None:
        assert _connect().execute("SELECT length(CURRENT_TIME)").fetchall() == [(8,)]

    def test_current_timestamp_length(self) -> None:
        assert _connect().execute(
            "SELECT length(CURRENT_TIMESTAMP)"
        ).fetchall() == [(19,)]

    def test_current_date_format(self) -> None:
        (val,) = _connect().execute("SELECT CURRENT_DATE").fetchall()[0]
        assert isinstance(val, str)
        assert re.fullmatch(r"\d{4}-\d{2}-\d{2}", val) is not None, val

    def test_current_time_format(self) -> None:
        (val,) = _connect().execute("SELECT CURRENT_TIME").fetchall()[0]
        assert isinstance(val, str)
        assert re.fullmatch(r"\d{2}:\d{2}:\d{2}", val) is not None, val

    def test_current_timestamp_format(self) -> None:
        (val,) = _connect().execute("SELECT CURRENT_TIMESTAMP").fetchall()[0]
        assert isinstance(val, str)
        assert (
            re.fullmatch(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}", val) is not None
        ), val


class TestCaseInsensitive:
    """SQL is case-insensitive for unquoted identifiers — all three spellings work."""

    def test_lowercase(self) -> None:
        assert _connect().execute(
            "SELECT length(current_timestamp)"
        ).fetchall() == [(19,)]

    def test_mixed_case(self) -> None:
        assert _connect().execute(
            "SELECT length(Current_Date)"
        ).fetchall() == [(10,)]


class TestInExpressionContext:
    def test_in_where_clause(self) -> None:
        # Always-true predicate just confirms the keyword evaluates to a
        # comparable string — not the actual date.
        assert _connect().execute(
            "SELECT 1 WHERE CURRENT_DATE >= '2000-01-01'"
        ).fetchall() == [(1,)]

    def test_in_case_branch(self) -> None:
        assert _connect().execute(
            "SELECT CASE WHEN CURRENT_DATE >= '2000-01-01' THEN 'ok' END"
        ).fetchall() == [("ok",)]

    def test_in_strftime(self) -> None:
        (val,) = _connect().execute(
            "SELECT strftime('%Y', CURRENT_DATE)"
        ).fetchall()[0]
        assert isinstance(val, str)
        assert re.fullmatch(r"\d{4}", val) is not None, val


class TestSqliteFormatCompat:
    """Format strings line up with sqlite3's — verified via length and shape."""

    def test_dates_match_sqlite_format(self) -> None:
        # Compare format only (not value — clocks differ on the microsecond
        # boundary).  Both should produce the same length and the same
        # number of dashes / colons / spaces.
        mini_val = _connect().execute("SELECT CURRENT_TIMESTAMP").fetchall()[0][0]
        ref_val = sqlite3.connect(":memory:").execute(
            "SELECT CURRENT_TIMESTAMP"
        ).fetchall()[0][0]
        assert len(mini_val) == len(ref_val)
        assert mini_val.count("-") == ref_val.count("-")
        assert mini_val.count(":") == ref_val.count(":")
        assert mini_val.count(" ") == ref_val.count(" ")


class TestKnownLimitation:
    """A user-created column literally named CURRENT_DATE is shadowed.

    SQLite resolves bareword ``CURRENT_DATE`` to the keyword even when
    a column with that exact name exists (the column is accessible only
    via the double-quoted form ``"CURRENT_DATE"``).  Mini-sqlite
    currently treats the double-quoted form the same way because the
    lexer's post-processing strips quotes from quoted identifiers and
    the adapter loses the "was quoted" distinction.  So this case
    diverges from SQLite — the quoted reference also resolves to the
    keyword.  Documented here so the divergence is visible.
    """

    def test_double_quoted_currentdate_diverges_from_sqlite(self) -> None:
        # SQLite returns the stored column value ('hello').
        # Mini-sqlite returns the date keyword value (today's date).
        # This test pins the *current* mini-sqlite behaviour so we notice
        # if a future fix unifies them.
        m = _connect()
        m.execute('CREATE TABLE t ("CURRENT_DATE" TEXT)')
        m.execute("INSERT INTO t VALUES ('hello')")
        (val,) = m.execute('SELECT "CURRENT_DATE" FROM t').fetchall()[0]
        # Mini-sqlite returns today's date string, not 'hello'.
        assert isinstance(val, str)
        assert re.fullmatch(r"\d{4}-\d{2}-\d{2}", val) is not None, val
