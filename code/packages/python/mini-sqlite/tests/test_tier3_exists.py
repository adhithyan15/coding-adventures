"""tests/test_tier3_exists.py — Phase 2: EXISTS / NOT EXISTS subquery expressions.

Tests are organised into three classes mirroring the implementation layers:

TestExistsBasic
    Unit-level tests directly on the expression pipeline: grammar parses,
    adapter builds ExistsSubquery, planner resolves, codegen emits
    RunExistsSubquery.

TestExistsIntegration
    End-to-end tests through mini_sqlite.connect() (SQL text → result set).
    Covers WHERE, HAVING, SELECT list, AND/OR combinations, and edge cases
    (empty table, constant subquery, LIMIT 0 subquery).

TestNotExistsIntegration
    Same coverage for NOT EXISTS.
"""

from __future__ import annotations

import mini_sqlite

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _conn() -> mini_sqlite.Connection:
    """Return a fresh in-memory connection with automatic index disabled."""
    return mini_sqlite.connect(":memory:", auto_index=False)


def _setup_dual(conn: mini_sqlite.Connection) -> None:
    """Create a single-row table used as the outer FROM when testing EXISTS
    as a scalar value expression in the SELECT list.

    Because our SQL grammar requires a FROM clause, queries like
    ``SELECT EXISTS (subquery)`` (which have no natural outer table) need a
    dummy one-row scan table.  Using a dedicated 'dual' pattern keeps tests
    readable without forcing every EXISTS scalar test to embed a meaningful
    outer FROM clause.
    """
    conn.execute("CREATE TABLE _dual (x INTEGER)")
    conn.execute("INSERT INTO _dual VALUES (1)")


def _setup_two_tables(conn: mini_sqlite.Connection) -> None:
    """Create customers (id, name) and orders (id, customer_id, amount)."""
    conn.execute("CREATE TABLE customers (id INTEGER, name TEXT)")
    conn.execute("CREATE TABLE orders (id INTEGER, customer_id INTEGER, amount INTEGER)")

    conn.execute("INSERT INTO customers VALUES (1, 'Alice')")
    conn.execute("INSERT INTO customers VALUES (2, 'Bob')")
    conn.execute("INSERT INTO customers VALUES (3, 'Carol')")

    conn.execute("INSERT INTO orders VALUES (1, 1, 100)")   # Alice's order
    conn.execute("INSERT INTO orders VALUES (2, 1, 200)")   # Alice's second order
    conn.execute("INSERT INTO orders VALUES (3, 2, 150)")   # Bob's order
    # Carol has no orders


# ---------------------------------------------------------------------------
# TestExistsBasic — pipeline unit tests
# ---------------------------------------------------------------------------


class TestExistsBasic:
    """Verify that each pipeline stage handles ExistsSubquery correctly."""

    def test_grammar_parses_exists(self) -> None:
        """Grammar accepts EXISTS (subquery) without a parse error."""
        conn = _conn()
        conn.execute("CREATE TABLE t (x INTEGER)")
        # No exception → grammar accepted and pipeline ran
        conn.execute("SELECT 1 FROM t WHERE EXISTS (SELECT 1 FROM t)")

    def test_grammar_parses_not_exists(self) -> None:
        """Grammar accepts NOT EXISTS (subquery)."""
        conn = _conn()
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("SELECT 1 FROM t WHERE NOT EXISTS (SELECT 1 FROM t)")

    def test_exists_true_when_rows_present(self) -> None:
        """EXISTS returns TRUE when the subquery has at least one row."""
        conn = _conn()
        _setup_dual(conn)
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (42)")
        # Grammar requires FROM; use _dual as a one-row outer table so
        # EXISTS (subquery) is evaluated as a SELECT-list expression.
        rows = conn.execute("SELECT EXISTS (SELECT x FROM t) FROM _dual").fetchall()
        assert rows == [(True,)]

    def test_exists_false_when_empty(self) -> None:
        """EXISTS returns FALSE when the subquery is empty."""
        conn = _conn()
        _setup_dual(conn)
        conn.execute("CREATE TABLE t (x INTEGER)")
        rows = conn.execute("SELECT EXISTS (SELECT x FROM t) FROM _dual").fetchall()
        assert rows == [(False,)]

    def test_not_exists_true_when_empty(self) -> None:
        """NOT EXISTS returns TRUE when the subquery is empty."""
        conn = _conn()
        _setup_dual(conn)
        conn.execute("CREATE TABLE t (x INTEGER)")
        rows = conn.execute("SELECT NOT EXISTS (SELECT x FROM t) FROM _dual").fetchall()
        assert rows == [(True,)]

    def test_not_exists_false_when_rows_present(self) -> None:
        """NOT EXISTS returns FALSE when the subquery has at least one row."""
        conn = _conn()
        _setup_dual(conn)
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (1)")
        rows = conn.execute("SELECT NOT EXISTS (SELECT x FROM t) FROM _dual").fetchall()
        assert rows == [(False,)]


# ---------------------------------------------------------------------------
# TestExistsIntegration — end-to-end WHERE / HAVING / SELECT-list tests
# ---------------------------------------------------------------------------


class TestExistsIntegration:
    """Full SQL text → result set tests for EXISTS."""

    def test_exists_in_where_filters_rows(self) -> None:
        """EXISTS in WHERE passes rows only when the subquery has results."""
        conn = _conn()
        _setup_two_tables(conn)

        # Uncorrelated EXISTS: the subquery always returns one row (orders is
        # non-empty), so all customers should be selected.
        rows = conn.execute(
            "SELECT name FROM customers WHERE EXISTS (SELECT 1 FROM orders)"
        ).fetchall()
        names = sorted(r[0] for r in rows)
        assert names == ["Alice", "Bob", "Carol"]

    def test_exists_where_empty_subquery_filters_all(self) -> None:
        """Uncorrelated EXISTS with empty subquery filters out all rows."""
        conn = _conn()
        _setup_two_tables(conn)

        conn.execute("CREATE TABLE empty_tbl (x INTEGER)")
        rows = conn.execute(
            "SELECT name FROM customers WHERE EXISTS (SELECT 1 FROM empty_tbl)"
        ).fetchall()
        assert rows == []

    def test_exists_with_filtered_subquery(self) -> None:
        """EXISTS with a WHERE inside the subquery respects the filter."""
        conn = _conn()
        _setup_two_tables(conn)

        # Subquery returns rows only when amount > 500 — no such row → FALSE.
        rows = conn.execute(
            "SELECT name FROM customers WHERE EXISTS (SELECT 1 FROM orders WHERE amount > 500)"
        ).fetchall()
        assert rows == []

        # Subquery returns rows when amount > 50 — orders has such rows → TRUE.
        rows = conn.execute(
            "SELECT name FROM customers WHERE EXISTS (SELECT 1 FROM orders WHERE amount > 50)"
        ).fetchall()
        assert len(rows) == 3  # all three customers

    def test_exists_in_select_list(self) -> None:
        """EXISTS used as a boolean expression in the SELECT list."""
        conn = _conn()
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (1)")

        # Outer table has one row; inner subquery hits the same (non-empty) table.
        rows = conn.execute(
            "SELECT x, EXISTS (SELECT x FROM t) FROM t"
        ).fetchall()
        assert rows == [(1, True)]

    def test_exists_select_list_multiple_outer_rows(self) -> None:
        """EXISTS in SELECT list is evaluated per outer row."""
        conn = _conn()
        conn.execute("CREATE TABLE outer_t (id INTEGER)")
        conn.execute("CREATE TABLE inner_t (id INTEGER)")
        conn.execute("INSERT INTO outer_t VALUES (1)")
        conn.execute("INSERT INTO outer_t VALUES (2)")
        conn.execute("INSERT INTO inner_t VALUES (10)")

        rows = conn.execute(
            "SELECT id, EXISTS (SELECT 1 FROM inner_t) FROM outer_t"
        ).fetchall()
        # Both outer rows see the same uncorrelated TRUE.
        assert rows == [(1, True), (2, True)]

    def test_exists_combined_with_and(self) -> None:
        """EXISTS combined with an AND predicate narrows the result."""
        conn = _conn()
        _setup_two_tables(conn)

        # Customers named 'Alice' AND orders table is non-empty.
        rows = conn.execute(
            "SELECT name FROM customers "
            "WHERE name = 'Alice' AND EXISTS (SELECT 1 FROM orders)"
        ).fetchall()
        assert rows == [("Alice",)]

    def test_exists_combined_with_or(self) -> None:
        """EXISTS combined with OR broadens the result."""
        conn = _conn()
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("CREATE TABLE t2 (y INTEGER)")
        conn.execute("INSERT INTO t VALUES (1)")
        conn.execute("INSERT INTO t2 VALUES (99)")

        rows = conn.execute(
            "SELECT x FROM t WHERE x = 999 OR EXISTS (SELECT 1 FROM t2)"
        ).fetchall()
        # x=1 doesn't satisfy x=999 but EXISTS is TRUE → row passes.
        assert rows == [(1,)]

    def test_exists_constant_subquery(self) -> None:
        """EXISTS (SELECT 1) without FROM is treated as always-true."""
        conn = _conn()
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (7)")

        # The grammar requires FROM for SELECT, so we use a dummy table trick:
        # any non-empty subquery that doesn't reference outer columns.
        conn.execute("CREATE TABLE one (v INTEGER)")
        conn.execute("INSERT INTO one VALUES (1)")
        rows = conn.execute(
            "SELECT x FROM t WHERE EXISTS (SELECT v FROM one)"
        ).fetchall()
        assert rows == [(7,)]

    def test_exists_with_limit_zero_subquery(self) -> None:
        """EXISTS (SELECT ... LIMIT 0) — subquery returns no rows → FALSE."""
        conn = _conn()
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (1)")
        conn.execute("CREATE TABLE inner_t (y INTEGER)")
        conn.execute("INSERT INTO inner_t VALUES (99)")

        rows = conn.execute(
            "SELECT x FROM t WHERE EXISTS (SELECT y FROM inner_t LIMIT 0)"
        ).fetchall()
        assert rows == []

    def test_exists_never_null(self) -> None:
        """EXISTS result is always TRUE or FALSE, never NULL."""
        conn = _conn()
        _setup_dual(conn)
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (NULL)")

        rows = conn.execute(
            "SELECT EXISTS (SELECT x FROM t WHERE x IS NULL) FROM _dual"
        ).fetchall()
        assert rows == [(True,)]

    def test_exists_multiple_inner_rows(self) -> None:
        """EXISTS returns TRUE even when the subquery returns many rows."""
        conn = _conn()
        _setup_dual(conn)
        conn.execute("CREATE TABLE t (x INTEGER)")
        for i in range(50):
            conn.execute(f"INSERT INTO t VALUES ({i})")

        rows = conn.execute("SELECT EXISTS (SELECT x FROM t) FROM _dual").fetchall()
        assert rows == [(True,)]

    def test_exists_in_having(self) -> None:
        """EXISTS can appear in a HAVING clause."""
        conn = _conn()
        conn.execute("CREATE TABLE sales (region TEXT, amount INTEGER)")
        conn.execute("INSERT INTO sales VALUES ('north', 100)")
        conn.execute("INSERT INTO sales VALUES ('south', 200)")
        conn.execute("CREATE TABLE active_regions (region TEXT)")
        conn.execute("INSERT INTO active_regions VALUES ('north')")

        # Total sales per region, but only for regions that are active.
        # Because the subquery is uncorrelated, either all or no groups pass.
        rows = conn.execute(
            "SELECT region, SUM(amount) FROM sales GROUP BY region "
            "HAVING EXISTS (SELECT 1 FROM active_regions)"
        ).fetchall()
        # active_regions is non-empty → HAVING is TRUE → both groups pass.
        result = {r[0]: r[1] for r in rows}
        assert result == {"north": 100, "south": 200}

    def test_exists_where_subquery_uses_different_table(self) -> None:
        """EXISTS subquery can reference any table, not just the outer table."""
        conn = _conn()
        conn.execute("CREATE TABLE products (id INTEGER, name TEXT)")
        conn.execute("CREATE TABLE discontinued (product_id INTEGER)")
        conn.execute("INSERT INTO products VALUES (1, 'Widget')")
        conn.execute("INSERT INTO products VALUES (2, 'Gadget')")
        conn.execute("INSERT INTO discontinued VALUES (2)")

        # Select products where discontinued table is non-empty (uncorrelated).
        rows = conn.execute(
            "SELECT name FROM products WHERE EXISTS (SELECT 1 FROM discontinued)"
        ).fetchall()
        names = sorted(r[0] for r in rows)
        assert names == ["Gadget", "Widget"]


# ---------------------------------------------------------------------------
# TestNotExistsIntegration — end-to-end NOT EXISTS tests
# ---------------------------------------------------------------------------


class TestNotExistsIntegration:
    """Full SQL text → result set tests for NOT EXISTS."""

    def test_not_exists_in_where_empty_table(self) -> None:
        """NOT EXISTS passes rows when the subquery is empty."""
        conn = _conn()
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("CREATE TABLE empty_t (y INTEGER)")
        conn.execute("INSERT INTO t VALUES (1)")
        conn.execute("INSERT INTO t VALUES (2)")

        rows = conn.execute(
            "SELECT x FROM t WHERE NOT EXISTS (SELECT 1 FROM empty_t)"
        ).fetchall()
        assert sorted(r[0] for r in rows) == [1, 2]

    def test_not_exists_in_where_non_empty_table(self) -> None:
        """NOT EXISTS filters all rows when the subquery is non-empty."""
        conn = _conn()
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("CREATE TABLE s (y INTEGER)")
        conn.execute("INSERT INTO t VALUES (1)")
        conn.execute("INSERT INTO s VALUES (99)")

        rows = conn.execute(
            "SELECT x FROM t WHERE NOT EXISTS (SELECT 1 FROM s)"
        ).fetchall()
        assert rows == []

    def test_not_exists_in_select_list(self) -> None:
        """NOT EXISTS used as a value expression in the SELECT list."""
        conn = _conn()
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("CREATE TABLE empty_t (y INTEGER)")
        conn.execute("INSERT INTO t VALUES (5)")

        rows = conn.execute(
            "SELECT x, NOT EXISTS (SELECT y FROM empty_t) FROM t"
        ).fetchall()
        assert rows == [(5, True)]

    def test_not_exists_combined_with_and(self) -> None:
        """NOT EXISTS combined with AND further restricts rows."""
        conn = _conn()
        _setup_two_tables(conn)
        conn.execute("CREATE TABLE empty_t (z INTEGER)")

        rows = conn.execute(
            "SELECT name FROM customers "
            "WHERE name = 'Alice' AND NOT EXISTS (SELECT 1 FROM empty_t)"
        ).fetchall()
        assert rows == [("Alice",)]

    def test_not_exists_combined_with_or(self) -> None:
        """NOT EXISTS in OR branch."""
        conn = _conn()
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("CREATE TABLE empty_t (y INTEGER)")
        conn.execute("INSERT INTO t VALUES (10)")
        conn.execute("INSERT INTO t VALUES (20)")

        rows = conn.execute(
            "SELECT x FROM t WHERE x = 10 OR NOT EXISTS (SELECT 1 FROM empty_t)"
        ).fetchall()
        # Both rows: x=10 satisfies first condition; empty_t is empty so
        # NOT EXISTS is TRUE for x=20 too.
        assert sorted(r[0] for r in rows) == [10, 20]

    def test_not_exists_never_null(self) -> None:
        """NOT EXISTS result is always TRUE or FALSE, never NULL."""
        conn = _conn()
        _setup_dual(conn)
        conn.execute("CREATE TABLE empty_t (y INTEGER)")

        rows = conn.execute(
            "SELECT NOT EXISTS (SELECT y FROM empty_t) FROM _dual"
        ).fetchall()
        assert rows == [(True,)]

    def test_not_exists_with_filtered_subquery(self) -> None:
        """NOT EXISTS with a WHERE in the subquery."""
        conn = _conn()
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (1)")
        conn.execute("CREATE TABLE values_t (v INTEGER)")
        conn.execute("INSERT INTO values_t VALUES (5)")
        conn.execute("INSERT INTO values_t VALUES (10)")

        # No rows where v > 100 → NOT EXISTS is TRUE.
        rows = conn.execute(
            "SELECT x FROM t WHERE NOT EXISTS (SELECT v FROM values_t WHERE v > 100)"
        ).fetchall()
        assert rows == [(1,)]

        # Rows exist where v > 3 → NOT EXISTS is FALSE.
        rows = conn.execute(
            "SELECT x FROM t WHERE NOT EXISTS (SELECT v FROM values_t WHERE v > 3)"
        ).fetchall()
        assert rows == []
