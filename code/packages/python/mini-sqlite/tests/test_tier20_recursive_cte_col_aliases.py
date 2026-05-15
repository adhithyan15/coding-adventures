"""
Tier-20 tests: WITH RECURSIVE CTE column alias list
====================================================

SQL allows a CTE to declare explicit column names for its output::

    WITH RECURSIVE cnt(n) AS (
        SELECT 1
        UNION ALL
        SELECT n + 1 FROM cnt WHERE n < 5
    )
    SELECT n FROM cnt;

Before this fix, the grammar's ``cte_def`` rule was::

    cte_def = NAME "AS" "(" query_stmt ")" ;

This rejected the ``(n)`` column list, producing a parse error at position 1.

**Grammar fix**: the rule is now::

    cte_def = NAME [ "(" NAME { "," NAME } ")" ] "AS" "(" query_stmt ")" ;

**Adapter fix**: when a column list is present, the adapter applies the
declared aliases to the anchor query's SELECT items so that the planner
derives the correct output column names.

Every test below is oracle-verified against the real ``sqlite3`` module.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite

# ---------------------------------------------------------------------------
# Helper
# ---------------------------------------------------------------------------


def _check(sql: str) -> None:
    """Assert mini_sqlite and sqlite3 produce the same rows for *sql*."""
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    got = mini.execute(sql).fetchall()
    exp = ref.execute(sql).fetchall()
    assert got == exp, f"SQL: {sql!r}\n  got {got}\n  exp {exp}"


# ---------------------------------------------------------------------------
# Basic counting CTEs with column alias list
# ---------------------------------------------------------------------------


def test_recursive_cte_with_single_column_alias():
    """Classic counter CTE with explicit column alias list."""
    _check("""
        WITH RECURSIVE cnt(n) AS (
            SELECT 1
            UNION ALL
            SELECT n + 1 FROM cnt WHERE n < 5
        )
        SELECT n FROM cnt
    """)


def test_recursive_cte_with_two_column_aliases():
    """CTE with two output columns, both declared via alias list."""
    _check("""
        WITH RECURSIVE seq(i, v) AS (
            SELECT 1, 10
            UNION ALL
            SELECT i + 1, v + 10 FROM seq WHERE i < 4
        )
        SELECT i, v FROM seq
    """)


def test_recursive_cte_fibonacci():
    """Fibonacci sequence via two-column alias list."""
    _check("""
        WITH RECURSIVE fib(a, b) AS (
            SELECT 0, 1
            UNION ALL
            SELECT b, a + b FROM fib WHERE a < 30
        )
        SELECT a FROM fib
    """)


def test_recursive_cte_column_alias_renamed_from_literal():
    """Rename a bare literal output (default name '1') to 'n' via alias list."""
    # SELECT 1 produces a column named '1' by default; the alias renames it 'n'.
    # The recursive step then references 'n', which must resolve correctly.
    _check("""
        WITH RECURSIVE nums(n) AS (
            SELECT 1
            UNION ALL
            SELECT n + 1 FROM nums WHERE n < 3
        )
        SELECT n FROM nums
    """)


def test_recursive_cte_union_not_all():
    """WITH RECURSIVE UNION (not UNION ALL) deduplicates between iterations."""
    _check("""
        WITH RECURSIVE cnt(n) AS (
            SELECT 1
            UNION
            SELECT (n + 1) % 4 FROM cnt WHERE n < 5
        )
        SELECT n FROM cnt ORDER BY n
    """)


# ---------------------------------------------------------------------------
# Column alias list without RECURSIVE
# ---------------------------------------------------------------------------


def test_non_recursive_cte_with_column_alias():
    """Non-recursive CTE with column alias list."""
    _check("WITH a(x) AS (SELECT 42) SELECT x FROM a")


def test_non_recursive_cte_with_two_column_aliases():
    """Non-recursive CTE with two column aliases."""
    _check("WITH pair(p, q) AS (SELECT 1, 2) SELECT p, q FROM pair")


def test_non_recursive_cte_alias_overrides_default_name():
    """Column alias overrides the default literal column name."""
    # 'SELECT 1 + 1' produces column '1 + 1' or similar without alias.
    # WITH a(result) AS (SELECT 1 + 1) renames it 'result'.
    _check("WITH a(result) AS (SELECT 1 + 1) SELECT result FROM a")


# ---------------------------------------------------------------------------
# Non-recursive CTE without column alias (regression guard)
# ---------------------------------------------------------------------------


def test_non_recursive_cte_without_alias_still_works():
    """WITH without column alias list must work exactly as before."""
    _check("WITH a AS (SELECT 42 AS x) SELECT x FROM a")


def test_recursive_cte_without_alias_still_works():
    """WITH RECURSIVE without column alias list must work exactly as before."""
    _check("""
        WITH RECURSIVE cnt AS (
            SELECT 1 AS n
            UNION ALL
            SELECT n + 1 FROM cnt WHERE n < 5
        )
        SELECT n FROM cnt
    """)


# ---------------------------------------------------------------------------
# Combined: ORDER BY and CTEs
# ---------------------------------------------------------------------------


def test_cte_with_order_by():
    """CTE results ordered by an output column."""
    _check("""
        WITH RECURSIVE cnt(n) AS (
            SELECT 5
            UNION ALL
            SELECT n - 1 FROM cnt WHERE n > 1
        )
        SELECT n FROM cnt ORDER BY n
    """)


def test_non_recursive_cte_joined_with_table():
    """Non-recursive CTE with column alias used in a JOIN."""
    mini = mini_sqlite.connect(":memory:")
    mini.execute("CREATE TABLE t (id INTEGER, val INTEGER)")
    mini.executemany("INSERT INTO t VALUES (?, ?)", [(1, 10), (2, 20), (3, 30)])
    mini.commit()

    ref = sqlite3.connect(":memory:")
    ref.execute("CREATE TABLE t (id INTEGER, val INTEGER)")
    ref.executemany("INSERT INTO t VALUES (?, ?)", [(1, 10), (2, 20), (3, 30)])

    sql = """
        WITH base(k) AS (SELECT 2)
        SELECT t.id, t.val FROM t JOIN base ON t.id = base.k
    """
    got = mini.execute(sql).fetchall()
    exp = ref.execute(sql).fetchall()
    assert got == exp
