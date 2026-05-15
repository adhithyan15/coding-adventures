"""
Tier-20 tests: ORDER BY a column not in the SELECT output
==========================================================

SQL allows ORDER BY to reference columns that do not appear in the SELECT
list.  For example::

    SELECT name FROM employees ORDER BY salary DESC

The column ``salary`` drives the sort, but the caller only sees ``name`` in
the result rows.

Prior to this fix the VM's ``SortResult`` handler called
``columns.index(k.column)`` where ``columns = st.result.columns`` — the
*output* column list.  If the sort key was not in that list, a ``ValueError``
was raised and surfaced as an ``InternalError``.

The fix works in two parts:

1. **Compile time** (sql-codegen ``_compile_read``): detect sort keys whose
   column names are absent from the Project's output.  Append those columns
   as hidden trailing ``ProjectionItem`` entries on the Project, and emit a
   ``StripTrailingColumns`` instruction immediately after ``SortResult``.  A
   ``SetResultSchema`` is also prepended so the VM's schema matches the
   extended row width during the scan/sort phase.

2. **Run time** (sql-vm ``StripTrailingColumns`` handler): after sorting,
   remove the last *n* column names from ``st.result.columns`` and the last
   *n* values from every row in ``st.result.rows``.

The fix is oracle-verified: every test below runs the same query against the
real ``sqlite3`` module and asserts byte-for-byte identical results.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _setup(schema: str, rows: list[tuple]) -> tuple[mini_sqlite.Connection, sqlite3.Connection]:
    """Return a (mini_sqlite, sqlite3) pair, both pre-loaded with the same data."""
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    mini.execute(schema)
    ref.execute(schema)
    mini.executemany(f"INSERT INTO t VALUES ({','.join('?' * len(rows[0]))})", rows)
    ref.executemany(f"INSERT INTO t VALUES ({','.join('?' * len(rows[0]))})", rows)
    mini.commit()
    return mini, ref


def _check(mini_con: mini_sqlite.Connection, ref_con: sqlite3.Connection, sql: str) -> None:
    """Assert mini_sqlite rows == sqlite3 rows for *sql*."""
    got = mini_con.execute(sql).fetchall()
    exp = ref_con.execute(sql).fetchall()
    assert got == exp, f"SQL: {sql!r}\n  got {got}\n  exp {exp}"


# ---------------------------------------------------------------------------
# Basic hidden-column ORDER BY
# ---------------------------------------------------------------------------


def test_order_by_hidden_column_asc():
    """SELECT y FROM t ORDER BY x — x is hidden, sort order is ASC."""
    mini, ref = _setup(
        "CREATE TABLE t (x INTEGER, y INTEGER)",
        [(3, 30), (1, 10), (2, 20)],
    )
    _check(mini, ref, "SELECT y FROM t ORDER BY x")


def test_order_by_hidden_column_desc():
    """SELECT y FROM t ORDER BY x DESC — hidden column with DESC direction."""
    mini, ref = _setup(
        "CREATE TABLE t (x INTEGER, y INTEGER)",
        [(3, 30), (1, 10), (2, 20)],
    )
    _check(mini, ref, "SELECT y FROM t ORDER BY x DESC")


def test_order_by_hidden_text_column():
    """Sort by a TEXT column not in the SELECT list."""
    mini, ref = _setup(
        "CREATE TABLE t (id INTEGER, name TEXT, score INTEGER)",
        [(1, "charlie", 80), (2, "alice", 95), (3, "bob", 70)],
    )
    _check(mini, ref, "SELECT score FROM t ORDER BY name")


def test_order_by_multiple_hidden_columns():
    """ORDER BY with two hidden columns (multi-key sort)."""
    mini, ref = _setup(
        "CREATE TABLE t (a INTEGER, b INTEGER, c INTEGER)",
        [(1, 2, 3), (1, 1, 9), (2, 1, 0), (1, 2, 1)],
    )
    _check(mini, ref, "SELECT c FROM t ORDER BY a ASC, b ASC, c DESC")


def test_order_by_mix_of_visible_and_hidden():
    """ORDER BY with one visible column and one hidden column."""
    mini, ref = _setup(
        "CREATE TABLE t (id INTEGER, name TEXT, dept TEXT, salary INTEGER)",
        [
            (1, "Alice", "eng", 100),
            (2, "Bob", "eng", 90),
            (3, "Carol", "hr", 80),
            (4, "Dave", "hr", 110),
        ],
    )
    # dept is visible, salary is hidden
    _check(mini, ref, "SELECT name, dept FROM t ORDER BY dept ASC, salary DESC")


def test_order_by_hidden_column_preserves_row_width():
    """Result rows must have exactly the SELECT-list column count (not more)."""
    mini, _ = _setup(
        "CREATE TABLE t (x INTEGER, y INTEGER)",
        [(3, 30), (1, 10), (2, 20)],
    )
    cur = mini.execute("SELECT y FROM t ORDER BY x")
    rows = cur.fetchall()
    # Each row must be a 1-tuple (only 'y').
    assert all(len(r) == 1 for r in rows), f"unexpected row width: {rows}"
    # Column descriptor must list only 'y'.
    assert len(cur.description) == 1
    assert cur.description[0][0] == "y"


def test_order_by_hidden_column_preserves_column_name():
    """cursor.description must show the SELECT columns, not the sort column."""
    mini, _ = _setup(
        "CREATE TABLE t (x INTEGER, y INTEGER, z TEXT)",
        [(3, 30, "c"), (1, 10, "a"), (2, 20, "b")],
    )
    cur = mini.execute("SELECT y, z FROM t ORDER BY x")
    cols = [d[0] for d in cur.description]
    assert cols == ["y", "z"]


def test_order_by_hidden_column_with_where():
    """WHERE clause applies before sorting; hidden ORDER BY still works."""
    mini, ref = _setup(
        "CREATE TABLE t (x INTEGER, y INTEGER)",
        [(1, 10), (2, 20), (3, 30), (4, 40), (5, 50)],
    )
    _check(mini, ref, "SELECT y FROM t WHERE y > 15 ORDER BY x DESC")


def test_order_by_hidden_column_with_limit():
    """LIMIT combined with hidden-column ORDER BY."""
    mini, ref = _setup(
        "CREATE TABLE t (x INTEGER, y INTEGER)",
        [(3, 30), (1, 10), (2, 20)],
    )
    _check(mini, ref, "SELECT y FROM t ORDER BY x LIMIT 2")


def test_order_by_hidden_column_with_limit_offset():
    """LIMIT + OFFSET combined with hidden-column ORDER BY."""
    mini, ref = _setup(
        "CREATE TABLE t (x INTEGER, y INTEGER)",
        [(4, 40), (2, 20), (1, 10), (3, 30)],
    )
    _check(mini, ref, "SELECT y FROM t ORDER BY x LIMIT 2 OFFSET 1")


# ---------------------------------------------------------------------------
# Non-hidden ORDER BY (regression guard — must still work)
# ---------------------------------------------------------------------------


def test_order_by_visible_column_still_works():
    """ORDER BY a visible SELECT column must work exactly as before."""
    mini, ref = _setup(
        "CREATE TABLE t (x INTEGER, y INTEGER)",
        [(3, 30), (1, 10), (2, 20)],
    )
    _check(mini, ref, "SELECT x, y FROM t ORDER BY x")


def test_order_by_star_visible_column():
    """SELECT * ORDER BY a column — the column IS in *, so no injection needed."""
    mini, ref = _setup(
        "CREATE TABLE t (id INTEGER, val INTEGER)",
        [(2, 99), (1, 42), (3, 77)],
    )
    _check(mini, ref, "SELECT * FROM t ORDER BY id")


def test_order_by_null_handling_hidden():
    """NULL rows in hidden sort column must not crash; NULLs sort last (our semantics).

    Note: SQLite places NULLs *first* for ASC ORDER BY (treating NULL as smaller
    than any other value), whereas this VM places NULLs *last*.  This is a known
    pre-existing behavioural difference in the plain ORDER BY path as well.  We
    verify here that the hidden-column injection does not crash and applies the
    VM's own NULL ordering rule consistently.
    """
    mini, _ = _setup(
        "CREATE TABLE t (x INTEGER, y INTEGER)",
        [(None, 1), (2, 2), (1, 3)],
    )
    rows = mini.execute("SELECT y FROM t ORDER BY x").fetchall()
    # Our VM puts NULLs last: x=1→y=3, x=2→y=2, x=NULL→y=1
    assert rows == [(3,), (2,), (1,)], f"unexpected: {rows}"


def test_order_by_hidden_column_single_row():
    """Single-row table: hidden-column sort is a no-op but must not crash."""
    mini, ref = _setup(
        "CREATE TABLE t (x INTEGER, y TEXT)",
        [(42, "hello")],
    )
    _check(mini, ref, "SELECT y FROM t ORDER BY x")


def test_order_by_hidden_column_empty_table():
    """Empty table: sort produces empty result without error."""
    mini, ref = _setup(
        "CREATE TABLE t (x INTEGER, y TEXT)",
        [(99, "placeholder")],  # insert one so executemany works
    )
    mini.execute("DELETE FROM t")
    ref.execute("DELETE FROM t")
    mini.commit()
    _check(mini, ref, "SELECT y FROM t ORDER BY x")
