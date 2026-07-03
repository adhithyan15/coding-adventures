"""End-to-end tests for ``sqlite_sequence`` synthesis.

SQLite exposes ``sqlite_sequence`` as an internal table that tracks
the high-water rowid for each AUTOINCREMENT table.  It materialises
lazily — querying it on a fresh database errors with "no such table"
until at least one AUTOINCREMENT table is declared.

Mini-sqlite synthesizes the rows on demand from the in-memory
backend's per-table ``_next_rowid`` counter.  Oracle-tested against
``sqlite3`` so the row content, lazy-materialization rule, and
read-only enforcement all match.
"""

from __future__ import annotations

import sqlite3

import pytest

import mini_sqlite
from mini_sqlite import errors as mini_errors


def _both_match(ddl: str, *dml: str, query: str) -> None:
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    for c in (mini, ref):
        c.execute(ddl)
        for d in dml:
            c.execute(d)
    assert mini.execute(query).fetchall() == ref.execute(query).fetchall()


class TestLazyMaterialization:
    """``sqlite_sequence`` only appears after an AUTOINCREMENT table exists."""

    def test_no_such_table_on_fresh_db(self) -> None:
        # Both engines reject the query identically.
        mini = mini_sqlite.connect(":memory:")
        ref = sqlite3.connect(":memory:")
        with pytest.raises(sqlite3.Error):
            ref.execute("SELECT * FROM sqlite_sequence").fetchall()
        with pytest.raises(mini_errors.Error):
            mini.execute("SELECT * FROM sqlite_sequence").fetchall()

    def test_plain_integer_pk_does_not_materialize(self) -> None:
        # INTEGER PRIMARY KEY *without* AUTOINCREMENT does not create
        # sqlite_sequence.
        mini = mini_sqlite.connect(":memory:")
        ref = sqlite3.connect(":memory:")
        for c in (mini, ref):
            c.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
            c.execute("INSERT INTO t(v) VALUES ('a')")
        with pytest.raises(sqlite3.Error):
            ref.execute("SELECT * FROM sqlite_sequence").fetchall()
        with pytest.raises(mini_errors.Error):
            mini.execute("SELECT * FROM sqlite_sequence").fetchall()


class TestRowContent:
    """High-water rowid is reported correctly per AUTOINCREMENT table."""

    def test_single_table_sequential_inserts(self) -> None:
        _both_match(
            "CREATE TABLE t (id INTEGER PRIMARY KEY AUTOINCREMENT, v TEXT)",
            "INSERT INTO t(v) VALUES ('a'),('b'),('c')",
            query="SELECT name, seq FROM sqlite_sequence",
        )

    def test_seq_survives_delete(self) -> None:
        _both_match(
            "CREATE TABLE t (id INTEGER PRIMARY KEY AUTOINCREMENT, v TEXT)",
            "INSERT INTO t(v) VALUES ('a'),('b'),('c')",
            "DELETE FROM t WHERE id = 3",
            query="SELECT name, seq FROM sqlite_sequence",
        )

    def test_multiple_autoincrement_tables(self) -> None:
        _both_match(
            "CREATE TABLE t1 (id INTEGER PRIMARY KEY AUTOINCREMENT, v TEXT)",
            "CREATE TABLE t2 (id INTEGER PRIMARY KEY AUTOINCREMENT, v TEXT)",
            "INSERT INTO t1(v) VALUES ('a'),('b')",
            "INSERT INTO t2(v) VALUES ('x')",
            query="SELECT name, seq FROM sqlite_sequence ORDER BY name",
        )

    def test_non_autoincr_table_excluded_from_listing(self) -> None:
        _both_match(
            "CREATE TABLE t (id INTEGER PRIMARY KEY AUTOINCREMENT, v TEXT)",
            "CREATE TABLE u (x INTEGER)",
            "INSERT INTO t(v) VALUES ('a')",
            "INSERT INTO u VALUES (10)",
            query="SELECT name FROM sqlite_sequence",
        )


class TestReadOnly:
    """Reserved name — CREATE / DROP / INSERT all fail."""

    def test_create_table_named_sqlite_sequence_rejected(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        with pytest.raises(mini_errors.Error):
            mini.execute("CREATE TABLE sqlite_sequence (a INT)")

    def test_drop_sqlite_sequence_rejected(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        with pytest.raises(mini_errors.Error):
            mini.execute("DROP TABLE sqlite_sequence")

    def test_insert_into_sqlite_sequence_rejected(self) -> None:
        # Once the table materializes via an AUTOINCREMENT declaration,
        # INSERT must still be rejected — SQLite's contract.
        mini = mini_sqlite.connect(":memory:")
        mini.execute("CREATE TABLE t (id INTEGER PRIMARY KEY AUTOINCREMENT)")
        with pytest.raises(mini_errors.Error):
            mini.execute("INSERT INTO sqlite_sequence VALUES ('fake', 99)")
