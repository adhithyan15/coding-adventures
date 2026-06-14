"""Oracle tests for ``SELECT *`` spanning multiple FROM sources.

This file pins a latent codegen bug fix where ``SELECT *`` only emitted
columns from the *first* opened cursor, causing cross-join queries
across N sources to return only the first table's columns instead of
all of them concatenated.

The fix is in ``sql_codegen/compiler.py`` — the Wildcard branch in
``_compile_project_body`` now iterates over every entry in
``ctx.alias_to_cursor`` (in insertion order) instead of taking only the
primary cursor via ``_primary_cursor``.

Why this matters: a lot of real-world SQLite SQL uses comma-separated
FROM clauses for joins, and assumes ``SELECT *`` returns all columns:

    SELECT * FROM orders, customers WHERE orders.customer_id = customers.id

Before the fix mini-sqlite returned only ``orders``' columns, silently
diverging from SQLite — confusing application bugs result.

All assertions compare against real ``sqlite3`` row-for-row.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _both(setup: list[str], query: str):
    """Apply *setup* statements then run *query* on both engines."""
    conn_m = mini_sqlite.connect(":memory:")
    conn_r = sqlite3.connect(":memory:")
    for s in setup:
        conn_m.execute(s)
        conn_r.execute(s)
    m = conn_m.execute(query).fetchall()
    r = conn_r.execute(query).fetchall()
    return m, r


def _check(setup: list[str], query: str) -> None:
    m, r = _both(setup, query)
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


def _check_no_setup(query: str) -> None:
    """Variant for queries that need no CREATE/INSERT setup."""
    _check([], query)


# ---------------------------------------------------------------------------
# Single source — sanity check (no behaviour change expected)
# ---------------------------------------------------------------------------


class TestSelectStarSingleSource:
    def test_single_table(self) -> None:
        _check(
            [
                "CREATE TABLE t (a INTEGER, b INTEGER)",
                "INSERT INTO t VALUES (1, 2), (3, 4)",
            ],
            "SELECT * FROM t ORDER BY a",
        )

    def test_single_derived_table(self) -> None:
        _check_no_setup("SELECT * FROM (SELECT 1 AS x, 2 AS y) t")


# ---------------------------------------------------------------------------
# Comma cross-join with plain tables
# ---------------------------------------------------------------------------


class TestSelectStarCommaCrossJoinTables:
    def test_two_tables(self) -> None:
        _check(
            [
                "CREATE TABLE a (x INTEGER)",
                "CREATE TABLE b (y INTEGER)",
                "INSERT INTO a VALUES (1)",
                "INSERT INTO b VALUES (2)",
            ],
            "SELECT * FROM a, b",
        )

    def test_three_tables(self) -> None:
        _check(
            [
                "CREATE TABLE a (x INTEGER)",
                "CREATE TABLE b (y INTEGER)",
                "CREATE TABLE c (z INTEGER)",
                "INSERT INTO a VALUES (1), (2)",
                "INSERT INTO b VALUES (10)",
                "INSERT INTO c VALUES (100)",
            ],
            "SELECT * FROM a, b, c ORDER BY a.x",
        )

    def test_with_where_predicate(self) -> None:
        _check(
            [
                "CREATE TABLE orders (id INTEGER, cust_id INTEGER, amount INTEGER)",
                "CREATE TABLE customers (id INTEGER, name TEXT)",
                "INSERT INTO orders VALUES (1, 100, 50), (2, 200, 75)",
                "INSERT INTO customers VALUES (100, 'Alice'), (200, 'Bob')",
            ],
            "SELECT * FROM orders, customers "
            "WHERE orders.cust_id = customers.id ORDER BY orders.id",
        )


# ---------------------------------------------------------------------------
# Comma cross-join with derived tables
# ---------------------------------------------------------------------------


class TestSelectStarCommaCrossJoinDerived:
    def test_two_derived_tables(self) -> None:
        _check_no_setup(
            "SELECT * FROM (SELECT 1 AS x) t1, (SELECT 2 AS y) t2"
        )

    def test_derived_table_and_real_table(self) -> None:
        _check(
            [
                "CREATE TABLE t (a INTEGER)",
                "INSERT INTO t VALUES (1)",
            ],
            "SELECT * FROM t, (SELECT 99 AS marker) m",
        )


# ---------------------------------------------------------------------------
# Comma cross-join with CTEs
# ---------------------------------------------------------------------------


class TestSelectStarCrossJoinCte:
    def test_two_ctes(self) -> None:
        _check_no_setup(
            "WITH x AS (SELECT 1 AS p), y AS (SELECT 2 AS q) "
            "SELECT * FROM x, y"
        )

    def test_three_ctes_mixed_with_real_table(self) -> None:
        _check(
            [
                "CREATE TABLE r (rr INTEGER)",
                "INSERT INTO r VALUES (99)",
            ],
            "WITH a AS (SELECT 1 AS aa), b AS (SELECT 2 AS bb) "
            "SELECT * FROM a, b, r",
        )


# ---------------------------------------------------------------------------
# Explicit JOIN ... ON (regression check — must still work)
# ---------------------------------------------------------------------------


class TestSelectStarExplicitJoinNoRegression:
    """The fix must not break SELECT * with explicit ``JOIN ON`` syntax."""

    def test_inner_join_on(self) -> None:
        _check(
            [
                "CREATE TABLE a (id INTEGER, name TEXT)",
                "CREATE TABLE b (id INTEGER, val INTEGER)",
                "INSERT INTO a VALUES (1, 'x'), (2, 'y')",
                "INSERT INTO b VALUES (1, 100), (2, 200)",
            ],
            "SELECT * FROM a JOIN b ON a.id = b.id ORDER BY a.id",
        )

    def test_left_join_on_matched_rows(self) -> None:
        """LEFT JOIN where every left row finds a right match.

        The unmatched-row case is covered by
        ``test_tier3_left_join_null_pad.py`` (the cursor-schema cache
        follow-up); this test guards the matched-row path against
        regression.
        """
        _check(
            [
                "CREATE TABLE a (id INTEGER, name TEXT)",
                "CREATE TABLE b (id INTEGER, val INTEGER)",
                "INSERT INTO a VALUES (1, 'x'), (2, 'y')",
                "INSERT INTO b VALUES (1, 100), (2, 200)",  # every a has a match
            ],
            "SELECT * FROM a LEFT JOIN b ON a.id = b.id ORDER BY a.id",
        )
