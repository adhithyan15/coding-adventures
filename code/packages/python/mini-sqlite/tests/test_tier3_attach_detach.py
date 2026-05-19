"""
``ATTACH DATABASE`` / ``DETACH DATABASE`` syntax.

SQLite uses ATTACH to mount additional databases under a schema name, then
``schema.table`` references them.  Mini-sqlite is a single-database engine,
so ATTACH and DETACH are accepted but no-op'd.

Grammar (SQLite-compatible):

  ATTACH [DATABASE] <expr> AS <name>
  DETACH [DATABASE] <name>

The optional ``DATABASE`` keyword may be omitted (SQLite accepts both forms).
``<expr>`` is the database file path; in real SQLite this is opened as a
new database.  In mini-sqlite the call returns silently without actually
attaching anything.

Limitations (intentional, documented):

* No multi-database query support — ``SELECT * FROM aux.t`` will fail
  because the planner doesn't know how to resolve the schema prefix.
* ``PRAGMA database_list`` still reports only ``main``.

Every test verifies the statement is *accepted* by both engines without
errors; behavioural cross-checks are not possible because the semantics
differ (mini-sqlite no-ops, real SQLite actually attaches).
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _both_accept(sql: str) -> None:
    """Both engines accept *sql* without raising."""
    mini_sqlite.connect(":memory:").execute(sql)
    sqlite3.connect(":memory:").execute(sql)


# ---------------------------------------------------------------------------
# ATTACH
# ---------------------------------------------------------------------------


def test_attach_database_with_keyword():
    _both_accept("ATTACH DATABASE ':memory:' AS aux")


def test_attach_without_database_keyword():
    """SQLite accepts ATTACH without the DATABASE keyword."""
    _both_accept("ATTACH ':memory:' AS aux")


def test_attach_case_insensitive():
    _both_accept("attach database ':memory:' as aux")


def test_attach_with_different_schema_name():
    _both_accept("ATTACH DATABASE ':memory:' AS my_aux_db")


# ---------------------------------------------------------------------------
# DETACH
# ---------------------------------------------------------------------------


def test_detach_database_with_keyword():
    mini = mini_sqlite.connect(":memory:")
    mini.execute("ATTACH DATABASE ':memory:' AS aux")
    mini.execute("DETACH DATABASE aux")


def test_detach_without_database_keyword():
    mini = mini_sqlite.connect(":memory:")
    mini.execute("ATTACH ':memory:' AS aux")
    mini.execute("DETACH aux")


# ---------------------------------------------------------------------------
# Common ORM patterns
# ---------------------------------------------------------------------------


def test_attach_then_detach_round_trip():
    """ORM code often attaches, queries, then detaches.  Verify the
    round-trip doesn't raise on the SQL-statement level."""
    mini = mini_sqlite.connect(":memory:")
    mini.execute("CREATE TABLE main_t (id INTEGER)")
    mini.execute("ATTACH DATABASE ':memory:' AS aux")
    # Queries against the main schema still work.
    mini.execute("INSERT INTO main_t VALUES (1)")
    assert mini.execute("SELECT COUNT(*) FROM main_t").fetchone() == (1,)
    mini.execute("DETACH DATABASE aux")
    # Post-detach, main queries continue to work.
    assert mini.execute("SELECT COUNT(*) FROM main_t").fetchone() == (1,)


def test_attach_returns_empty_result():
    """ATTACH/DETACH are DDL — they return no rows."""
    mini = mini_sqlite.connect(":memory:")
    cur = mini.execute("ATTACH DATABASE ':memory:' AS aux")
    assert cur.fetchall() == []


def test_multiple_attaches_succeed():
    """Attaching multiple aliases in sequence is accepted."""
    mini = mini_sqlite.connect(":memory:")
    mini.execute("ATTACH DATABASE ':memory:' AS aux1")
    mini.execute("ATTACH DATABASE ':memory:' AS aux2")
    mini.execute("DETACH DATABASE aux1")
    mini.execute("DETACH DATABASE aux2")
