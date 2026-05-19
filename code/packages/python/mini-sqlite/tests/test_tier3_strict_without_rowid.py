"""
``STRICT`` and ``WITHOUT ROWID`` table options on ``CREATE TABLE``.

SQLite 3.37+ added the ``STRICT`` keyword to enforce strict typing per
column; SQLite 3.8.2+ added ``WITHOUT ROWID`` to store rows in the
primary-key B-tree.  ORMs and migration tools commonly emit these clauses.

Mini-sqlite accepts both syntaxes and silently ignores them:

  CREATE TABLE t (id INTEGER) STRICT
  CREATE TABLE t (id INTEGER PRIMARY KEY) WITHOUT ROWID
  CREATE TABLE t (id INTEGER PRIMARY KEY) STRICT, WITHOUT ROWID
  CREATE TABLE t (id INTEGER PRIMARY KEY) WITHOUT ROWID, STRICT

Limitations (intentional, documented):

* STRICT does NOT enforce strict typing — mini-sqlite uses lenient SQLite
  type affinity regardless of the STRICT marker.
* WITHOUT ROWID is a pure no-op — the storage model is unchanged.

Test strategy: each test verifies both engines accept the CREATE TABLE
plus a follow-up INSERT/SELECT.  We don't oracle-compare strict-typing
behaviour because mini-sqlite intentionally diverges.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _both_accept(create_sql: str, *follow_ups: str) -> None:
    """Both engines accept *create_sql* and the follow-up statements."""
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    for c in (mini, ref):
        c.execute(create_sql)
        for sql in follow_ups:
            c.execute(sql)


# ---------------------------------------------------------------------------
# STRICT
# ---------------------------------------------------------------------------


def test_strict_table_basic():
    _both_accept("CREATE TABLE t (id INTEGER, name TEXT) STRICT")


def test_strict_table_with_primary_key():
    _both_accept(
        "CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT) STRICT",
        "INSERT INTO t VALUES (1, 'alice')",
    )


def test_strict_table_with_inserts():
    """STRICT table accepts INSERT and SELECT roundtrips."""
    mini = mini_sqlite.connect(":memory:")
    mini.execute("CREATE TABLE t (id INTEGER, val INTEGER) STRICT")
    mini.execute("INSERT INTO t VALUES (1, 10), (2, 20)")
    assert mini.execute("SELECT id, val FROM t ORDER BY id").fetchall() == \
        [(1, 10), (2, 20)]


# ---------------------------------------------------------------------------
# WITHOUT ROWID
# ---------------------------------------------------------------------------


def test_without_rowid_table_basic():
    _both_accept("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT) WITHOUT ROWID")


def test_without_rowid_with_inserts():
    """WITHOUT ROWID table accepts INSERT and SELECT roundtrips."""
    mini = mini_sqlite.connect(":memory:")
    mini.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val INTEGER) WITHOUT ROWID")
    mini.execute("INSERT INTO t VALUES (1, 10), (2, 20)")
    assert mini.execute("SELECT id, val FROM t ORDER BY id").fetchall() == \
        [(1, 10), (2, 20)]


def test_without_rowid_case_insensitive():
    _both_accept("CREATE TABLE t (id INTEGER PRIMARY KEY) without rowid")


# ---------------------------------------------------------------------------
# Combined: STRICT + WITHOUT ROWID
# ---------------------------------------------------------------------------


def test_strict_then_without_rowid():
    _both_accept("CREATE TABLE t (id INTEGER PRIMARY KEY) STRICT, WITHOUT ROWID")


def test_without_rowid_then_strict():
    _both_accept("CREATE TABLE t (id INTEGER PRIMARY KEY) WITHOUT ROWID, STRICT")


# ---------------------------------------------------------------------------
# Regression: ROWID as a column reference still works
# ---------------------------------------------------------------------------


def test_rowid_as_column_reference_still_works():
    """``rowid`` is not a reserved keyword; it remains usable as a column
    reference in SELECT statements (a SQLite pseudo-column)."""
    mini = mini_sqlite.connect(":memory:")
    mini.execute("CREATE TABLE t (val INTEGER)")
    mini.execute("INSERT INTO t VALUES (10), (20)")
    rows = mini.execute("SELECT rowid, val FROM t ORDER BY rowid").fetchall()
    assert rows == [(1, 10), (2, 20)]


def test_create_table_without_options_still_works():
    """Plain CREATE TABLE without any table options must still parse."""
    mini = mini_sqlite.connect(":memory:")
    mini.execute("CREATE TABLE t (id INTEGER)")
    mini.execute("INSERT INTO t VALUES (42)")
    assert mini.execute("SELECT * FROM t").fetchone() == (42,)


# ---------------------------------------------------------------------------
# Common ORM / migration patterns
# ---------------------------------------------------------------------------


def test_strict_with_check_constraint():
    _both_accept(
        "CREATE TABLE t (id INTEGER CHECK(id > 0), name TEXT NOT NULL) STRICT"
    )


def test_strict_with_default_values():
    """STRICT table with DEFAULT clauses."""
    _both_accept(
        "CREATE TABLE t (id INTEGER PRIMARY KEY, score REAL DEFAULT 0.0) STRICT"
    )
