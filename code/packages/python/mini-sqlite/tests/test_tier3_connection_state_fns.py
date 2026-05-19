"""
Connection-state scalar functions: ``changes()``, ``total_changes()``,
``last_insert_rowid()``, ``sqlite_version()``, ``sqlite_source_id()``.

These functions all return values that depend on connection state rather
than their arguments, so they're tricky to oracle-compare for the
*version* functions (mini-sqlite reports its own version string).  We
oracle-compare the connection-counter functions and shape-check the
version functions.

Truth table:

+-------------------------+----------------------------------------+
| Function                | Returns                                |
+=========================+========================================+
| ``changes()``           | rows affected by the most recent       |
|                         | INSERT / UPDATE / DELETE               |
| ``total_changes()``     | cumulative rows affected since         |
|                         | connection opened                      |
| ``last_insert_rowid()`` | rowid of the most recent INSERT, or 0  |
| ``sqlite_version()``    | a dotted-integer version string        |
| ``sqlite_source_id()``  | a build-identifier string              |
+-------------------------+----------------------------------------+
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _setup_two():
    """Return ``(mini, ref)`` connections with the same fresh table."""
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    for c in (mini, ref):
        c.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val INTEGER)")
    return mini, ref


def _both(sql: str):
    mini, ref = _setup_two()
    return (
        mini.execute(sql).fetchone(),
        ref.execute(sql).fetchone(),
    )


# ---------------------------------------------------------------------------
# changes() / total_changes() after INSERT
# ---------------------------------------------------------------------------


def test_changes_after_single_insert():
    mini, ref = _setup_two()
    for c in (mini, ref):
        c.execute("INSERT INTO t VALUES (1, 10)")
    assert mini.execute("SELECT changes()").fetchone() == \
           ref.execute("SELECT changes()").fetchone()


def test_changes_after_multi_value_insert():
    mini, ref = _setup_two()
    for c in (mini, ref):
        c.execute("INSERT INTO t VALUES (1, 10), (2, 20), (3, 30)")
    assert mini.execute("SELECT changes()").fetchone() == \
           ref.execute("SELECT changes()").fetchone()


def test_total_changes_accumulates():
    mini, ref = _setup_two()
    for c in (mini, ref):
        c.execute("INSERT INTO t VALUES (1, 10)")
        c.execute("INSERT INTO t VALUES (2, 20)")
    assert mini.execute("SELECT total_changes()").fetchone() == \
           ref.execute("SELECT total_changes()").fetchone()


def test_total_changes_includes_updates():
    mini, ref = _setup_two()
    for c in (mini, ref):
        c.execute("INSERT INTO t VALUES (1, 10), (2, 20)")
        c.execute("UPDATE t SET val = val * 2")
    assert mini.execute("SELECT total_changes()").fetchone() == \
           ref.execute("SELECT total_changes()").fetchone()


def test_changes_after_update():
    mini, ref = _setup_two()
    for c in (mini, ref):
        c.execute("INSERT INTO t VALUES (1, 10), (2, 20)")
        c.execute("UPDATE t SET val = 99")
    assert mini.execute("SELECT changes()").fetchone() == \
           ref.execute("SELECT changes()").fetchone()


def test_changes_after_delete():
    mini, ref = _setup_two()
    for c in (mini, ref):
        c.execute("INSERT INTO t VALUES (1, 10), (2, 20), (3, 30)")
        c.execute("DELETE FROM t WHERE val > 10")
    assert mini.execute("SELECT changes()").fetchone() == \
           ref.execute("SELECT changes()").fetchone()


# ---------------------------------------------------------------------------
# last_insert_rowid()
# ---------------------------------------------------------------------------


def test_last_insert_rowid_integer_pk():
    """For INTEGER PRIMARY KEY tables, last_insert_rowid is the PK value."""
    mini, ref = _setup_two()
    for c in (mini, ref):
        c.execute("INSERT INTO t VALUES (5, 100)")
    assert mini.execute("SELECT last_insert_rowid()").fetchone() == \
           ref.execute("SELECT last_insert_rowid()").fetchone()


def test_last_insert_rowid_picks_up_latest():
    """A later INSERT overwrites the rowid value."""
    mini, ref = _setup_two()
    for c in (mini, ref):
        c.execute("INSERT INTO t VALUES (1, 10)")
        c.execute("INSERT INTO t VALUES (7, 70)")
    assert mini.execute("SELECT last_insert_rowid()").fetchone() == \
           ref.execute("SELECT last_insert_rowid()").fetchone()


def test_last_insert_rowid_zero_before_any_insert():
    """Fresh connection: last_insert_rowid() returns 0."""
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    assert mini.execute("SELECT last_insert_rowid()").fetchone() == \
           ref.execute("SELECT last_insert_rowid()").fetchone()


# ---------------------------------------------------------------------------
# Version functions — shape check (not oracle-compare since values differ)
# ---------------------------------------------------------------------------


def test_sqlite_version_returns_string():
    mini = mini_sqlite.connect(":memory:")
    v = mini.execute("SELECT sqlite_version()").fetchone()[0]
    assert isinstance(v, str)
    # Format: dotted integers (at least major.minor).
    parts = v.split(".")
    assert len(parts) >= 2
    assert all(p.isdigit() for p in parts)


def test_sqlite_version_parseable():
    mini = mini_sqlite.connect(":memory:")
    v = mini.execute("SELECT sqlite_version()").fetchone()[0]
    # Common idiom: applications gate on version tuples.
    parts = tuple(int(p) for p in v.split("."))
    assert parts >= (3, 0, 0)


def test_sqlite_source_id_returns_string():
    mini = mini_sqlite.connect(":memory:")
    s = mini.execute("SELECT sqlite_source_id()").fetchone()[0]
    assert isinstance(s, str)
    assert len(s) > 0
