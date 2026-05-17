"""
SQLite-compatible NULL ordering in ORDER BY.

SQLite treats NULL as the *smallest* value when sorting.  Therefore:

  • ASC  → NULL rows appear first
  • DESC → NULL rows appear last

This is the SQL:2003 default for systems that use NULL-as-smallest semantics
(PostgreSQL uses the opposite default but allows NULLS FIRST/LAST overrides).

Truth table:

+-------------------+----------------------+
| ORDER BY x        | NULL position        |
+===================+======================+
| ASC (default)     | first                |
| DESC              | last                 |
| ASC NULLS LAST    | last  (explicit)     |
| DESC NULLS FIRST  | first (explicit)     |
+-------------------+----------------------+

Every test in this file oracle-compares against the real ``sqlite3`` module.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _setup(rows):
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    schema = "CREATE TABLE t (id INTEGER, val INTEGER)"
    mini.execute(schema)
    ref.execute(schema)
    mini.executemany("INSERT INTO t VALUES (?, ?)", rows)
    ref.executemany("INSERT INTO t VALUES (?, ?)", rows)
    return mini, ref


def _check(mini, ref, sql):
    got = mini.execute(sql).fetchall()
    exp = ref.execute(sql).fetchall()
    assert got == exp, f"SQL: {sql!r}\n  got {got}\n  exp {exp}"


def test_asc_nulls_appear_first():
    """ASC default: NULL rows lead the result."""
    mini, ref = _setup([(1, 10), (2, None), (3, 30), (4, None), (5, 20)])
    _check(mini, ref, "SELECT id FROM t ORDER BY val")


def test_desc_nulls_appear_last():
    """DESC default: NULL rows trail the result."""
    mini, ref = _setup([(1, 10), (2, None), (3, 30), (4, None), (5, 20)])
    _check(mini, ref, "SELECT id FROM t ORDER BY val DESC")


def test_asc_multiple_nulls():
    """Multiple NULLs in ASC sort stay grouped at the front."""
    mini, ref = _setup([(1, None), (2, 5), (3, None), (4, 1), (5, None)])
    _check(mini, ref, "SELECT id FROM t ORDER BY val")


def test_desc_multiple_nulls():
    """Multiple NULLs in DESC sort stay grouped at the back."""
    mini, ref = _setup([(1, None), (2, 5), (3, None), (4, 1), (5, None)])
    _check(mini, ref, "SELECT id FROM t ORDER BY val DESC")


def test_asc_all_nulls():
    """ASC sort when every row is NULL: order should be by insertion (stable)."""
    mini, ref = _setup([(1, None), (2, None), (3, None)])
    _check(mini, ref, "SELECT id FROM t ORDER BY val")


def test_asc_no_nulls():
    """ASC sort with no NULLs is unchanged by the new rule."""
    mini, ref = _setup([(1, 30), (2, 10), (3, 20)])
    _check(mini, ref, "SELECT id, val FROM t ORDER BY val")


def test_asc_nulls_with_limit():
    """NULLs go first in ASC; LIMIT may pull out only NULL rows."""
    mini, ref = _setup([(1, 100), (2, None), (3, 50), (4, None)])
    _check(mini, ref, "SELECT id FROM t ORDER BY val LIMIT 2")


def test_desc_nulls_with_limit():
    """LIMIT after DESC sort: top-N excludes the NULLs at the end."""
    mini, ref = _setup([(1, 100), (2, None), (3, 50), (4, None)])
    _check(mini, ref, "SELECT id FROM t ORDER BY val DESC LIMIT 2")


def test_multi_key_with_nulls():
    """Multi-key ORDER BY: NULL ordering rule applies per key."""
    mini, ref = _setup([(1, None), (2, 5), (3, None), (4, 5), (5, None)])
    _check(mini, ref, "SELECT id FROM t ORDER BY val ASC, id DESC")


def test_order_by_text_column_with_nulls():
    """NULL ordering also applies to TEXT columns."""
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    mini.execute("CREATE TABLE t (id INTEGER, name TEXT)")
    ref.execute("CREATE TABLE t (id INTEGER, name TEXT)")
    rows = [(1, "carol"), (2, None), (3, "alice"), (4, None), (5, "bob")]
    mini.executemany("INSERT INTO t VALUES (?, ?)", rows)
    ref.executemany("INSERT INTO t VALUES (?, ?)", rows)
    got = mini.execute("SELECT id FROM t ORDER BY name").fetchall()
    exp = ref.execute("SELECT id FROM t ORDER BY name").fetchall()
    assert got == exp, f"got {got}\n  exp {exp}"
