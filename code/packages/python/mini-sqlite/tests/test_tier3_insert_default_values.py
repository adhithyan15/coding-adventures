"""Tests for ``INSERT INTO t DEFAULT VALUES``.

SQLite's ``DEFAULT VALUES`` shorthand inserts a single row consisting
entirely of column defaults — equivalent to ``INSERT INTO t () VALUES
()``.  Useful for tables where every column either has a DEFAULT
clause or is NULLable / auto-assigned.

Mini-sqlite previously parse-errored on the syntax.  This PR adds:

* Grammar: ``insert_body = "VALUES" row_value … | "DEFAULT" "VALUES"
  | query_stmt``.
* Adapter: detects the ``DEFAULT`` keyword inside ``insert_body`` and
  emits ``InsertValuesStmt(rows=((),), columns=())`` — the empty
  tuple of values triggers the existing ``_apply_defaults`` /
  ``_autoassign_ipk`` path in the backend.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _both_match(*stmts: str, query: str) -> None:
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    for c in (mini, ref):
        for s in stmts:
            c.execute(s)
    assert mini.execute(query).fetchall() == ref.execute(query).fetchall()


class TestBasic:
    def test_default_values_with_ipk(self) -> None:
        _both_match(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)",
            "INSERT INTO t DEFAULT VALUES",
            query="SELECT * FROM t",
        )

    def test_default_values_uses_explicit_defaults(self) -> None:
        _both_match(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT DEFAULT 'x', n INT DEFAULT 99)",
            "INSERT INTO t DEFAULT VALUES",
            query="SELECT * FROM t",
        )

    def test_default_values_nullable_columns(self) -> None:
        _both_match(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, a TEXT, b INT)",
            "INSERT INTO t DEFAULT VALUES",
            query="SELECT * FROM t",
        )


class TestSequentialInserts:
    def test_three_inserts_increment_ipk(self) -> None:
        _both_match(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT DEFAULT 'x')",
            "INSERT INTO t DEFAULT VALUES",
            "INSERT INTO t DEFAULT VALUES",
            "INSERT INTO t DEFAULT VALUES",
            query="SELECT id FROM t ORDER BY id",
        )


class TestReturning:
    def test_default_values_returning_star(self) -> None:
        _both_match(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT DEFAULT 'x')",
            query="INSERT INTO t DEFAULT VALUES RETURNING *",
        )

    def test_default_values_returning_id(self) -> None:
        _both_match(
            "CREATE TABLE t (id INTEGER PRIMARY KEY)",
            query="INSERT INTO t DEFAULT VALUES RETURNING id",
        )


class TestNotNullWithoutDefault:
    """A NOT NULL column without a DEFAULT must still violate."""

    def test_not_null_violation(self) -> None:
        import pytest

        from mini_sqlite import errors as mini_errors

        mini = mini_sqlite.connect(":memory:")
        mini.execute(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT NOT NULL)"
        )
        with pytest.raises(mini_errors.IntegrityError):
            mini.execute("INSERT INTO t DEFAULT VALUES")
