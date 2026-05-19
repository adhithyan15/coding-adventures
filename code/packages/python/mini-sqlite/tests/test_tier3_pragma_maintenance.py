"""
Maintenance PRAGMAs: ``optimize``, ``integrity_check``, ``quick_check``.

These pragmas trigger heavy work in real SQLite — analyzing statistics,
walking the B-tree, verifying constraints.  Mini-sqlite holds everything
in memory, so there is nothing to optimise and nothing to corrupt.  We
return the same shape the real ``sqlite3`` module returns for a healthy
database:

  PRAGMA optimize           → empty result
  PRAGMA optimize(0)        → empty result   (argument is ignored)
  PRAGMA optimize(N)        → empty result   (argument is a bitmask)
  PRAGMA integrity_check    → ``[('ok',)]``
  PRAGMA integrity_check(N) → ``[('ok',)]``  (N = max errors to report)
  PRAGMA integrity_check(T) → ``[('ok',)]``  (T = restrict to one table)
  PRAGMA quick_check        → ``[('ok',)]``

Every assertion oracle-compares against the real ``sqlite3`` module.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _both(sql: str):
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    for c in (mini, ref):
        c.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val INTEGER)")
        c.execute("INSERT INTO t VALUES (1, 10), (2, 20), (3, 30)")
    return (
        mini.execute(sql).fetchall(),
        ref.execute(sql).fetchall(),
    )


def _check(sql: str) -> None:
    m, r = _both(sql)
    assert m == r, f"SQL: {sql!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# optimize
# ---------------------------------------------------------------------------


def test_optimize_no_args():
    _check("PRAGMA optimize")


def test_optimize_zero():
    _check("PRAGMA optimize(0)")


def test_optimize_bitmask():
    _check("PRAGMA optimize(15)")


# ---------------------------------------------------------------------------
# integrity_check
# ---------------------------------------------------------------------------


def test_integrity_check_no_args():
    _check("PRAGMA integrity_check")


def test_integrity_check_max_errors():
    _check("PRAGMA integrity_check(10)")


def test_integrity_check_table_arg():
    _check("PRAGMA integrity_check('t')")


def test_integrity_check_table_double_quoted():
    _check('PRAGMA integrity_check("t")')


# ---------------------------------------------------------------------------
# quick_check
# ---------------------------------------------------------------------------


def test_quick_check_no_args():
    _check("PRAGMA quick_check")


def test_quick_check_max_errors():
    _check("PRAGMA quick_check(10)")


# ---------------------------------------------------------------------------
# Common ORM/Alembic usage
# ---------------------------------------------------------------------------


def test_integrity_check_returns_single_row():
    """ORM code typically asserts the result is one row with 'ok'."""
    mini = mini_sqlite.connect(":memory:")
    mini.execute("CREATE TABLE t (id INTEGER)")
    rows = mini.execute("PRAGMA integrity_check").fetchall()
    assert rows == [("ok",)]


def test_quick_check_returns_single_row():
    mini = mini_sqlite.connect(":memory:")
    mini.execute("CREATE TABLE t (id INTEGER)")
    rows = mini.execute("PRAGMA quick_check").fetchall()
    assert rows == [("ok",)]


def test_optimize_returns_empty_result():
    mini = mini_sqlite.connect(":memory:")
    mini.execute("CREATE TABLE t (id INTEGER)")
    rows = mini.execute("PRAGMA optimize").fetchall()
    assert rows == []
