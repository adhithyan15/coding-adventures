"""End-to-end tests for ``INTEGER PRIMARY KEY`` auto-rowid assignment.

SQLite treats ``INTEGER PRIMARY KEY`` as an alias for the rowid: when
INSERT omits the column or passes ``NULL``, SQLite assigns the next
rowid; when the user supplies an explicit integer, the rowid counter
bumps past it so subsequent auto-assigns don't collide.

Mini-sqlite previously rejected the omit/NULL forms with a NOT NULL
violation — making ORM-style ``INSERT INTO t(name) VALUES ('alice')``
fail.  These tests verify the new auto-assign behaviour is
SQLite-compatible and oracle-tested against the stdlib driver.
"""

from __future__ import annotations

import sqlite3

import pytest

import mini_sqlite
from mini_sqlite import errors as mini_errors


def _both_match(ddl: str, *dml: str, query: str) -> None:
    """Both engines must produce identical results for *query* after *ddl* + *dml*."""
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    for c in (mini, ref):
        c.execute(ddl)
        for d in dml:
            c.execute(d)
    assert mini.execute(query).fetchall() == ref.execute(query).fetchall()


class TestOmitColumn:
    """``INSERT INTO t(other_col) VALUES (...)`` auto-assigns the id."""

    def test_single_insert(self) -> None:
        _both_match(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)",
            "INSERT INTO t(v) VALUES ('a')",
            query="SELECT * FROM t",
        )

    def test_multiple_inserts_get_sequential_ids(self) -> None:
        _both_match(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)",
            "INSERT INTO t(v) VALUES ('a'), ('b'), ('c')",
            query="SELECT id, v FROM t ORDER BY id",
        )


class TestExplicitNull:
    """``INSERT INTO t VALUES (NULL, ...)`` auto-assigns the id."""

    def test_null_id(self) -> None:
        _both_match(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)",
            "INSERT INTO t VALUES (NULL, 'a')",
            query="SELECT * FROM t",
        )


class TestExplicitValue:
    """An explicit integer id is stored verbatim and bumps the counter."""

    def test_explicit_id_stored_verbatim(self) -> None:
        _both_match(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)",
            "INSERT INTO t VALUES (42, 'a')",
            query="SELECT * FROM t",
        )

    def test_explicit_then_auto_bumps_counter(self) -> None:
        # The auto-assign after an explicit 100 must yield 101 — matching
        # SQLite's ``_next_rowid = max(_next_rowid, supplied_id + 1)`` rule.
        _both_match(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)",
            "INSERT INTO t VALUES (100, 'a')",
            "INSERT INTO t(v) VALUES ('b')",
            query="SELECT id, v FROM t ORDER BY id",
        )


class TestRowidAlias:
    """``rowid`` and the INTEGER PRIMARY KEY column return identical values."""

    def test_rowid_equals_id(self) -> None:
        _both_match(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)",
            "INSERT INTO t(v) VALUES ('a'), ('b'), ('c')",
            query="SELECT rowid, id FROM t ORDER BY id",
        )


class TestLastInsertRowid:
    """``last_insert_rowid()`` reflects the auto-assigned value."""

    def test_after_omit_returns_assigned_id(self) -> None:
        _both_match(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)",
            "INSERT INTO t(v) VALUES ('a')",
            query="SELECT last_insert_rowid()",
        )

    def test_after_explicit_returns_explicit_id(self) -> None:
        _both_match(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)",
            "INSERT INTO t VALUES (50, 'a')",
            query="SELECT last_insert_rowid()",
        )


class TestNonIntegerPkNoAutoAssign:
    """Only INTEGER PRIMARY KEY gets rowid-alias treatment — TEXT PK doesn't."""

    def test_text_pk_null_still_violates(self) -> None:
        # TEXT PRIMARY KEY is *not* a rowid alias.  NULL on a TEXT PK
        # must continue to violate the implicit NOT NULL.
        mini = mini_sqlite.connect(":memory:")
        mini.execute("CREATE TABLE t (name TEXT PRIMARY KEY, v TEXT)")
        with pytest.raises(mini_errors.IntegrityError):
            mini.execute("INSERT INTO t VALUES (NULL, 'x')")


class TestColumnOrderPreserved:
    """Auto-assigned id appears in the correct column position.

    Regression guard: the original implementation accidentally built the
    row dict in caller-insertion order, putting the auto-assigned id at
    the END of the dict.  ``SELECT *`` walks the dict in insertion
    order, so the user saw ``(v, id)`` instead of ``(id, v)``.  The
    fix builds the row dict in column-declaration order regardless of
    which columns the caller supplied.
    """

    def test_select_star_returns_id_first(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        mini.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
        mini.execute("INSERT INTO t(v) VALUES ('a')")
        assert mini.execute("SELECT * FROM t").fetchall() == [(1, "a")]
