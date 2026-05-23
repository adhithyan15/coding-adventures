"""Tests for the PRAGMA table_info shape fixes (mini-sqlite 1.97+).

Two discrepancies vs real ``sqlite3`` are fixed:

1. ``notnull`` column was previously ``1`` for ``id INTEGER PRIMARY KEY``
   (PK-implied NOT NULL).  SQLite reports ``0`` and only sets ``1``
   when the user wrote both ``PRIMARY KEY`` and ``NOT NULL``.
2. ``dflt_value`` was the parsed Python value.  SQLite returns the
   literal source text — ``DEFAULT 42`` → ``'42'``, ``DEFAULT 'x'``
   → ``"'x'"``, ``DEFAULT NULL`` → ``'NULL'``, etc.

Oracle-tested against ``sqlite3`` for byte compatibility.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _both(ddl: str) -> tuple:
    """Return (mini_rows, ref_rows) for PRAGMA table_info(t) after *ddl*."""
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    for c in (mini, ref):
        c.execute(ddl)
    return (
        mini.execute("PRAGMA table_info(t)").fetchall(),
        ref.execute("PRAGMA table_info(t)").fetchall(),
    )


class TestNotnullShape:
    """``notnull`` distinguishes explicit-NOT-NULL from PK-implied."""

    def test_ipk_without_explicit_not_null(self) -> None:
        m, r = _both("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
        assert m == r
        assert m[0][3] == 0  # notnull=0 — PK alone doesn't set it

    def test_ipk_with_explicit_not_null(self) -> None:
        m, r = _both("CREATE TABLE t (id INTEGER PRIMARY KEY NOT NULL, v TEXT)")
        assert m == r
        assert m[0][3] == 1  # notnull=1 — user wrote it explicitly

    def test_text_pk(self) -> None:
        # Same rule for non-INTEGER PRIMARY KEY.
        m, r = _both("CREATE TABLE t (id TEXT PRIMARY KEY, v TEXT)")
        assert m == r
        assert m[0][3] == 0

    def test_explicit_not_null_on_non_pk(self) -> None:
        # Non-PK columns: NOT NULL is always explicit, so notnull
        # tracks the raw declaration directly.
        m, r = _both("CREATE TABLE t (id INTEGER, name TEXT NOT NULL)")
        assert m == r
        assert m[0][3] == 0  # id: nullable
        assert m[1][3] == 1  # name: explicitly NOT NULL


class TestDefaultValueShape:
    """``dflt_value`` returns the SQL-literal source text, not Python value."""

    def test_string_default_keeps_quotes(self) -> None:
        m, r = _both("CREATE TABLE t (v TEXT DEFAULT 'hello')")
        assert m == r
        assert m[0][4] == "'hello'"

    def test_int_default_as_text(self) -> None:
        m, r = _both("CREATE TABLE t (n INT DEFAULT 42)")
        assert m == r
        assert m[0][4] == "42"

    def test_real_default_as_text(self) -> None:
        m, r = _both("CREATE TABLE t (r REAL DEFAULT 3.14)")
        assert m == r
        assert m[0][4] == "3.14"

    def test_null_default_as_text(self) -> None:
        m, r = _both("CREATE TABLE t (x INT DEFAULT NULL)")
        assert m == r
        assert m[0][4] == "NULL"

    def test_no_default_is_none(self) -> None:
        # When there's no DEFAULT clause, dflt_value is Python None
        # (which the DB-API surfaces as NULL).
        m, r = _both("CREATE TABLE t (x INT)")
        assert m == r
        assert m[0][4] is None


class TestEnforcementUnchanged:
    """The shape fixes don't change runtime NOT NULL enforcement."""

    def test_pk_still_rejects_explicit_null_for_non_integer_pk(self) -> None:
        # TEXT PRIMARY KEY: PK still implies NOT NULL at runtime,
        # so INSERT VALUES (NULL) is rejected (the rowid auto-assign
        # path only fires for INTEGER PRIMARY KEY).
        import pytest

        from mini_sqlite import errors as mini_errors

        mini = mini_sqlite.connect(":memory:")
        mini.execute("CREATE TABLE t (id TEXT PRIMARY KEY, v TEXT)")
        with pytest.raises(mini_errors.IntegrityError):
            mini.execute("INSERT INTO t VALUES (NULL, 'x')")

    def test_ipk_auto_assigns_unchanged(self) -> None:
        # INTEGER PRIMARY KEY still auto-assigns on omit/NULL.
        mini = mini_sqlite.connect(":memory:")
        mini.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
        mini.execute("INSERT INTO t(v) VALUES ('a')")
        assert mini.execute("SELECT * FROM t").fetchall() == [(1, "a")]
