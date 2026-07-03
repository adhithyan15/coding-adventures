"""Tests for ``INTEGER PRIMARY KEY AUTOINCREMENT``.

SQLite's AUTOINCREMENT clause may follow ``PRIMARY KEY`` on an
``INTEGER`` column.  Its semantic effect: rowids never reuse a value
that has been deleted (monotonic sequence forever).  Mini-sqlite's
in-memory backend already has this property because ``_next_rowid``
is never decremented — the new grammar/adapter wiring just lets the
keyword parse so common ORM CREATE TABLE statements work.

Test strategy:

* Parser accepts ``PRIMARY KEY AUTOINCREMENT``.
* Inserts auto-assign sequential rowids (inherits from the IPK
  auto-assign path).
* After DELETE, the next inserted row gets the next-rowid, not the
  recycled one — mirrors SQLite's monotonic-rowid guarantee.
* The CREATE statement round-trips through ``sqlite_master.sql`` with
  the AUTOINCREMENT keyword preserved.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _both_match(ddl: str, *dml: str, query: str) -> None:
    """Both engines must agree on *query* after *ddl* + *dml*."""
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    for c in (mini, ref):
        c.execute(ddl)
        for d in dml:
            c.execute(d)
    assert mini.execute(query).fetchall() == ref.execute(query).fetchall()


class TestParse:
    """Parser accepts the keyword in all its expected positions."""

    def test_basic_pkey_autoincrement(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        mini.execute(
            "CREATE TABLE t (id INTEGER PRIMARY KEY AUTOINCREMENT, v TEXT)"
        )

    def test_autoincrement_with_other_columns(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        mini.execute(
            "CREATE TABLE users ("
            "id INTEGER PRIMARY KEY AUTOINCREMENT, "
            "name TEXT NOT NULL, "
            "email TEXT UNIQUE"
            ")"
        )


class TestInsertBehaviour:
    """Inserts get sequential rowids, matching the IPK auto-assign path."""

    def test_sequential_ids(self) -> None:
        _both_match(
            "CREATE TABLE t (id INTEGER PRIMARY KEY AUTOINCREMENT, v TEXT)",
            "INSERT INTO t(v) VALUES ('a')",
            "INSERT INTO t(v) VALUES ('b')",
            "INSERT INTO t(v) VALUES ('c')",
            query="SELECT * FROM t ORDER BY id",
        )

    def test_monotonic_after_delete(self) -> None:
        # SQLite's AUTOINCREMENT promise: the deleted rowid (3) is NOT
        # reused — the next insert gets id=4.
        _both_match(
            "CREATE TABLE t (id INTEGER PRIMARY KEY AUTOINCREMENT, v TEXT)",
            "INSERT INTO t(v) VALUES ('a')",
            "INSERT INTO t(v) VALUES ('b')",
            "INSERT INTO t(v) VALUES ('c')",
            "DELETE FROM t WHERE id = 3",
            "INSERT INTO t(v) VALUES ('d')",
            query="SELECT * FROM t ORDER BY id",
        )


class TestSqlMasterRoundTrip:
    """The AUTOINCREMENT keyword survives reconstruction in sqlite_master.sql."""

    def test_keyword_appears_in_sqlite_master_sql(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        mini.execute(
            "CREATE TABLE t (id INTEGER PRIMARY KEY AUTOINCREMENT, v TEXT)"
        )
        row = mini.execute(
            "SELECT sql FROM sqlite_master WHERE name = 't'"
        ).fetchone()
        assert row is not None
        sql = row[0]
        assert "AUTOINCREMENT" in sql

    def test_no_autoincrement_when_not_declared(self) -> None:
        # A regular INTEGER PRIMARY KEY (no AUTOINCREMENT) should NOT
        # have the keyword in its reconstructed sql.
        mini = mini_sqlite.connect(":memory:")
        mini.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
        row = mini.execute(
            "SELECT sql FROM sqlite_master WHERE name = 't'"
        ).fetchone()
        assert row is not None
        assert "AUTOINCREMENT" not in row[0]
