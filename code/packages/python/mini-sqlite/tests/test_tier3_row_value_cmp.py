"""
Row-value comparison: ``(a, b) op (c, d)`` and ``(a, b) IN ((x, y), …)``.

SQLite extends the ISO SQL row-value comparison syntax to allow multi-column
predicates in WHERE and other expression contexts:

  WHERE (a, b) = (1, 2)          -- equality
  WHERE (a, b) < (3, 4)          -- lexicographic order
  WHERE (a, b) IN ((1,2),(3,4))  -- membership

Expansion rules (matching SQLite's semantics):

  ``=``   → ``a=x AND b=y AND …``
  ``!=``  → ``a!=x OR b!=y OR …``  (any column differs → not equal)
  ``<``   → ``a<x OR (a=x AND b<y) OR …``  (lexicographic)
  ``<=``  → ``a<x OR (a=x AND b<=y) OR …``
  ``>``   → ``a>x OR (a=x AND b>y) OR …``
  ``>=``  → ``a>x OR (a=x AND b>=y) OR …``
  ``IN``  → ``(a=x AND b=y) OR (a=p AND b=q) OR …``
  ``NOT IN`` → negation of the above

Implementation note: the grammar's ``comparison`` rule was extended with three
new PEG alternatives that fire before the scalar ``collated [...]`` form.  The
adapter expands each row-value comparison into an equivalent scalar BinaryExpr
tree so the planner and VM require no changes.

Scalar comparisons using parenthesised single-element expressions such as
``(a) = 1`` or ``(a) IN (1, 2)`` are tested to confirm they still work via
the existing scalar path (they fall through to the ``collated`` alternative).
"""

from __future__ import annotations

import sqlite3

import mini_sqlite

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _mini(ddl: list[str]) -> mini_sqlite.Connection:
    conn = mini_sqlite.connect(":memory:")
    for stmt in ddl:
        conn.execute(stmt)
    return conn


def _real(ddl: list[str]) -> sqlite3.Connection:
    conn = sqlite3.connect(":memory:")
    for stmt in ddl:
        conn.execute(stmt)
    return conn


_BASE_DDL = [
    "CREATE TABLE t (a INTEGER, b INTEGER)",
    "INSERT INTO t VALUES (1, 2)",
    "INSERT INTO t VALUES (1, 3)",
    "INSERT INTO t VALUES (2, 1)",
    "INSERT INTO t VALUES (2, 2)",
    "INSERT INTO t VALUES (3, 0)",
]


def _rows(conn: mini_sqlite.Connection | sqlite3.Connection, sql: str) -> set[tuple[int, int]]:
    return set(conn.execute(sql).fetchall())


# ---------------------------------------------------------------------------
# Equality / inequality
# ---------------------------------------------------------------------------


def test_row_value_eq():
    """``(a, b) = (x, y)`` selects the single matching row."""
    m = _mini(_BASE_DDL)
    r = _real(_BASE_DDL)
    sql = "SELECT a, b FROM t WHERE (a, b) = (2, 1)"
    assert _rows(m, sql) == _rows(r, sql) == {(2, 1)}


def test_row_value_ne():
    """``(a, b) != (x, y)`` excludes the matching row, returns all others."""
    m = _mini(_BASE_DDL)
    r = _real(_BASE_DDL)
    sql = "SELECT a, b FROM t WHERE (a, b) != (2, 1)"
    assert _rows(m, sql) == _rows(r, sql)
    assert (2, 1) not in _rows(m, sql)
    assert len(_rows(m, sql)) == 4


# ---------------------------------------------------------------------------
# Ordered comparisons (lexicographic)
# ---------------------------------------------------------------------------


def test_row_value_lt():
    """``(a, b) < (x, y)`` uses lexicographic ordering."""
    m = _mini(_BASE_DDL)
    r = _real(_BASE_DDL)
    sql = "SELECT a, b FROM t WHERE (a, b) < (2, 1)"
    assert _rows(m, sql) == _rows(r, sql) == {(1, 2), (1, 3)}


def test_row_value_gt():
    """``(a, b) > (x, y)`` uses lexicographic ordering."""
    m = _mini(_BASE_DDL)
    r = _real(_BASE_DDL)
    sql = "SELECT a, b FROM t WHERE (a, b) > (2, 1)"
    assert _rows(m, sql) == _rows(r, sql) == {(2, 2), (3, 0)}


def test_row_value_le():
    """``(a, b) <= (x, y)`` includes the boundary row."""
    m = _mini(_BASE_DDL)
    r = _real(_BASE_DDL)
    sql = "SELECT a, b FROM t WHERE (a, b) <= (2, 1)"
    assert _rows(m, sql) == _rows(r, sql) == {(1, 2), (1, 3), (2, 1)}


def test_row_value_ge():
    """``(a, b) >= (x, y)`` includes the boundary row."""
    m = _mini(_BASE_DDL)
    r = _real(_BASE_DDL)
    sql = "SELECT a, b FROM t WHERE (a, b) >= (2, 1)"
    assert _rows(m, sql) == _rows(r, sql) == {(2, 1), (2, 2), (3, 0)}


def test_row_value_lt_first_column_dominates():
    """First column determines order when it differs."""
    m = _mini(_BASE_DDL)
    r = _real(_BASE_DDL)
    sql = "SELECT a, b FROM t WHERE (a, b) < (2, 0)"
    assert _rows(m, sql) == _rows(r, sql) == {(1, 2), (1, 3)}


def test_row_value_three_columns():
    """Lexicographic comparison works for 3-column row values."""
    ddl = [
        "CREATE TABLE u (x INTEGER, y INTEGER, z INTEGER)",
        "INSERT INTO u VALUES (1, 2, 3)",
        "INSERT INTO u VALUES (1, 2, 4)",
        "INSERT INTO u VALUES (1, 3, 0)",
        "INSERT INTO u VALUES (2, 0, 0)",
    ]
    m = _mini(ddl)
    r = _real(ddl)
    sql = "SELECT x, y, z FROM u WHERE (x, y, z) < (1, 2, 4)"
    assert set(m.execute(sql).fetchall()) == set(r.execute(sql).fetchall()) == {(1, 2, 3)}


# ---------------------------------------------------------------------------
# IN / NOT IN with row values
# ---------------------------------------------------------------------------


def test_row_value_in():
    """``(a, b) IN ((x, y), …)`` returns rows matching any candidate."""
    m = _mini(_BASE_DDL)
    r = _real(_BASE_DDL)
    sql = "SELECT a, b FROM t WHERE (a, b) IN ((1, 2), (3, 0))"
    assert _rows(m, sql) == _rows(r, sql) == {(1, 2), (3, 0)}


def test_row_value_not_in():
    """``(a, b) NOT IN ((x, y), …)`` excludes matching rows."""
    m = _mini(_BASE_DDL)
    r = _real(_BASE_DDL)
    sql = "SELECT a, b FROM t WHERE (a, b) NOT IN ((1, 2), (3, 0))"
    assert _rows(m, sql) == _rows(r, sql)
    assert len(_rows(m, sql)) == 3


def test_row_value_in_single_candidate():
    """IN with exactly one candidate is equivalent to equality."""
    m = _mini(_BASE_DDL)
    r = _real(_BASE_DDL)
    sql = "SELECT a, b FROM t WHERE (a, b) IN ((2, 2))"
    assert _rows(m, sql) == _rows(r, sql) == {(2, 2)}


def test_row_value_in_no_match():
    """IN with candidates that match nothing returns no rows."""
    m = _mini(_BASE_DDL)
    r = _real(_BASE_DDL)
    sql = "SELECT a, b FROM t WHERE (a, b) IN ((9, 9), (8, 8))"
    assert _rows(m, sql) == _rows(r, sql) == set()


# ---------------------------------------------------------------------------
# Row-value in UPDATE WHERE clause
# ---------------------------------------------------------------------------


def test_row_value_in_update():
    """Row-value comparison works in UPDATE … WHERE."""
    m = _mini(_BASE_DDL)
    m.execute("UPDATE t SET b = 99 WHERE (a, b) = (1, 2)")
    rows = set(m.execute("SELECT a, b FROM t").fetchall())
    assert (1, 99) in rows
    assert (1, 2) not in rows


# ---------------------------------------------------------------------------
# Regression: scalar comparisons via paren still work
# ---------------------------------------------------------------------------


def test_scalar_paren_eq_literal():
    """``(a) = 1`` still resolves as scalar comparison (paren around column)."""
    m = _mini(_BASE_DDL)
    r = _real(_BASE_DDL)
    sql = "SELECT a, b FROM t WHERE (a) = 1"
    assert _rows(m, sql) == _rows(r, sql)


def test_scalar_paren_in_list():
    """``(a) IN (1, 2)`` still works as scalar IN with parenthesised operand."""
    m = _mini(_BASE_DDL)
    r = _real(_BASE_DDL)
    sql = "SELECT a, b FROM t WHERE (a) IN (1, 3)"
    assert _rows(m, sql) == _rows(r, sql)


def test_scalar_eq_unchanged():
    """Plain scalar ``a = 1`` is unaffected by the grammar extension."""
    m = _mini(_BASE_DDL)
    r = _real(_BASE_DDL)
    sql = "SELECT a, b FROM t WHERE a = 1"
    assert _rows(m, sql) == _rows(r, sql)


def test_scalar_lt_unchanged():
    """Plain scalar ``a < 2`` is unaffected."""
    m = _mini(_BASE_DDL)
    r = _real(_BASE_DDL)
    sql = "SELECT a, b FROM t WHERE a < 2"
    assert _rows(m, sql) == _rows(r, sql)
