"""
Phase 5a: Non-recursive Common Table Expressions (WITH clause).

Tests are organised in three classes:

  TestCTEGrammar       — grammar and adapter pipeline unit tests
  TestCTEIntegration   — end-to-end SQL correctness tests via mini-sqlite
  TestCTEErrors        — unaffected-error and edge-case tests
"""

from __future__ import annotations

import pytest

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_conn() -> object:
    """Open an in-memory mini-sqlite connection."""
    from mini_sqlite import connect  # type: ignore[import]

    return connect(":memory:")


def _setup_tables(conn: object) -> None:
    """Create standard fixtures: customers and orders."""
    conn.execute(
        "CREATE TABLE customers (id INTEGER PRIMARY KEY, name TEXT, region TEXT)"
    )
    conn.execute(
        "CREATE TABLE orders "
        "(id INTEGER PRIMARY KEY, customer_id INTEGER, amount INTEGER)"
    )
    conn.execute("INSERT INTO customers VALUES (1, 'Alice', 'east')")
    conn.execute("INSERT INTO customers VALUES (2, 'Bob',   'west')")
    conn.execute("INSERT INTO customers VALUES (3, 'Carol', 'east')")
    conn.execute("INSERT INTO orders VALUES (1, 1, 100)")
    conn.execute("INSERT INTO orders VALUES (2, 1, 200)")
    conn.execute("INSERT INTO orders VALUES (3, 2, 50)")
    conn.execute("INSERT INTO orders VALUES (4, 3, 75)")


# ---------------------------------------------------------------------------
# TestCTEGrammar — pipeline unit tests
# ---------------------------------------------------------------------------


class TestCTEGrammar:
    """Verify that WITH / CTE syntax parses and the adapter resolves the names."""

    def test_grammar_parses_cte_basic(self) -> None:
        """WITH c AS (SELECT ...) SELECT ... parses to a valid AST."""
        from sql_parser import parse_sql  # type: ignore[import]

        sql = "WITH c AS (SELECT 1 AS n FROM t) SELECT n FROM c"
        tree = parse_sql(sql)
        assert tree is not None
        assert tree.rule_name == "program"

    def test_grammar_parses_multiple_ctes(self) -> None:
        """Multiple comma-separated CTEs parse without error."""
        from sql_parser import parse_sql  # type: ignore[import]

        sql = (
            "WITH a AS (SELECT id FROM t), "
            "b AS (SELECT id FROM t WHERE id > 1) "
            "SELECT id FROM b"
        )
        tree = parse_sql(sql)
        assert tree is not None

    def test_adapter_resolves_cte_to_derived_table_ref(self) -> None:
        """Adapter converts a CTE reference in FROM into a DerivedTableRef."""
        from sql_parser import parse_sql  # type: ignore[import]
        from sql_planner.ast import DerivedTableRef, SelectStmt  # type: ignore[import]

        from mini_sqlite.adapter import to_statement  # type: ignore[import]

        sql = "WITH c AS (SELECT id FROM t) SELECT id FROM c"
        tree = parse_sql(sql)
        stmt = to_statement(tree)
        assert isinstance(stmt, SelectStmt)
        assert isinstance(stmt.from_, DerivedTableRef)
        assert stmt.from_.alias == "c"

    def test_adapter_cte_alias_default(self) -> None:
        """CTE used as FROM without explicit AS gets the CTE name as alias."""
        from sql_parser import parse_sql  # type: ignore[import]
        from sql_planner.ast import DerivedTableRef, SelectStmt  # type: ignore[import]

        from mini_sqlite.adapter import to_statement  # type: ignore[import]

        sql = "WITH my_cte AS (SELECT id FROM t) SELECT id FROM my_cte"
        tree = parse_sql(sql)
        stmt = to_statement(tree)
        assert isinstance(stmt, SelectStmt)
        assert isinstance(stmt.from_, DerivedTableRef)
        assert stmt.from_.alias == "my_cte"

    def test_adapter_cte_with_explicit_alias(self) -> None:
        """CTE used as FROM with 'AS alias' uses the explicit alias."""
        from sql_parser import parse_sql  # type: ignore[import]
        from sql_planner.ast import DerivedTableRef, SelectStmt  # type: ignore[import]

        from mini_sqlite.adapter import to_statement  # type: ignore[import]

        sql = "WITH c AS (SELECT id FROM t) SELECT c2.id FROM c AS c2"
        tree = parse_sql(sql)
        stmt = to_statement(tree)
        assert isinstance(stmt, SelectStmt)
        assert isinstance(stmt.from_, DerivedTableRef)
        assert stmt.from_.alias == "c2"


# ---------------------------------------------------------------------------
# TestCTEIntegration — end-to-end SQL correctness
# ---------------------------------------------------------------------------


class TestCTEIntegration:
    """Full end-to-end tests through the mini-sqlite Connection / Cursor."""

    def test_cte_basic(self) -> None:
        """Simple CTE returns correct rows."""
        conn = _make_conn()
        _setup_tables(conn)
        rows = conn.execute(
            "WITH big_orders AS (SELECT id, customer_id, amount FROM orders WHERE amount >= 100) "
            "SELECT id, amount FROM big_orders"
        ).fetchall()
        amounts = sorted(r[1] for r in rows)
        assert amounts == [100, 200]

    def test_cte_with_filter(self) -> None:
        """Outer WHERE clause applied on top of CTE results."""
        conn = _make_conn()
        _setup_tables(conn)
        rows = conn.execute(
            "WITH all_orders AS (SELECT id, amount FROM orders) "
            "SELECT id FROM all_orders WHERE amount > 75"
        ).fetchall()
        ids = sorted(r[0] for r in rows)
        assert ids == [1, 2]

    def test_cte_with_aggregation(self) -> None:
        """CTE body containing GROUP BY."""
        conn = _make_conn()
        _setup_tables(conn)
        rows = conn.execute(
            "WITH totals AS ( "
            "    SELECT customer_id, SUM(amount) AS total "
            "    FROM orders "
            "    GROUP BY customer_id "
            ") "
            "SELECT customer_id, total FROM totals WHERE total > 100"
        ).fetchall()
        by_cid = {r[0]: r[1] for r in rows}
        assert by_cid == {1: 300}

    def test_cte_multiple(self) -> None:
        """Multiple CTEs: later CTE references earlier CTE."""
        conn = _make_conn()
        _setup_tables(conn)
        rows = conn.execute(
            "WITH east_customers AS ( "
            "    SELECT id FROM customers WHERE region = 'east' "
            "), "
            "east_orders AS ( "
            "    SELECT o.amount FROM orders AS o "
            "    INNER JOIN east_customers AS ec ON o.customer_id = ec.id "
            ") "
            "SELECT amount FROM east_orders"
        ).fetchall()
        amounts = sorted(r[0] for r in rows)
        assert amounts == [75, 100, 200]

    def test_cte_joined_with_real_table(self) -> None:
        """CTE joined against a base table."""
        conn = _make_conn()
        _setup_tables(conn)
        rows = conn.execute(
            "WITH large AS ( "
            "    SELECT customer_id, amount FROM orders WHERE amount >= 100 "
            ") "
            "SELECT c.name, l.amount "
            "FROM large AS l "
            "INNER JOIN customers AS c ON l.customer_id = c.id"
        ).fetchall()
        pairs = sorted(rows)
        assert ("Alice", 100) in pairs
        assert ("Alice", 200) in pairs

    def test_cte_column_alias_preserved(self) -> None:
        """Aliased columns in the CTE body are accessible in the outer query."""
        conn = _make_conn()
        conn.execute("CREATE TABLE nums (n INTEGER)")
        conn.execute("INSERT INTO nums VALUES (3)")
        conn.execute("INSERT INTO nums VALUES (7)")
        rows = conn.execute(
            "WITH doubled AS (SELECT n * 2 AS d FROM nums) "
            "SELECT d FROM doubled"
        ).fetchall()
        values = sorted(r[0] for r in rows)
        assert values == [6, 14]

    def test_cte_outer_alias(self) -> None:
        """CTE referenced in FROM with an explicit outer alias."""
        conn = _make_conn()
        _setup_tables(conn)
        rows = conn.execute(
            "WITH cheap AS (SELECT id, amount FROM orders WHERE amount <= 75) "
            "SELECT x.amount FROM cheap AS x"
        ).fetchall()
        amounts = sorted(r[0] for r in rows)
        assert amounts == [50, 75]

    def test_cte_in_join_position(self) -> None:
        """CTE appears on the right-hand side of a JOIN."""
        conn = _make_conn()
        _setup_tables(conn)
        rows = conn.execute(
            "WITH west AS (SELECT id FROM customers WHERE region = 'west') "
            "SELECT o.id, o.amount "
            "FROM orders AS o "
            "INNER JOIN west AS w ON o.customer_id = w.id"
        ).fetchall()
        assert len(rows) == 1
        assert rows[0][1] == 50

    def test_plain_query_unaffected(self) -> None:
        """Plain SELECT without WITH still works correctly."""
        conn = _make_conn()
        _setup_tables(conn)
        rows = conn.execute("SELECT id FROM customers ORDER BY id").fetchall()
        assert [r[0] for r in rows] == [1, 2, 3]


# ---------------------------------------------------------------------------
# TestCTEErrors — error cases and edge conditions
# ---------------------------------------------------------------------------


class TestCTEErrors:
    def test_unknown_table_still_fails(self) -> None:
        """Non-CTE table not found still raises OperationalError."""
        from mini_sqlite import OperationalError  # type: ignore[import]

        conn = _make_conn()
        with pytest.raises(OperationalError):
            conn.execute("SELECT id FROM no_such_table").fetchall()

    def test_cte_body_can_be_complex_select(self) -> None:
        """CTE body may use DISTINCT, ORDER BY, LIMIT."""
        conn = _make_conn()
        _setup_tables(conn)
        rows = conn.execute(
            "WITH top2 AS ( "
            "    SELECT DISTINCT amount FROM orders ORDER BY amount DESC LIMIT 2 "
            ") "
            "SELECT amount FROM top2"
        ).fetchall()
        amounts = sorted(r[0] for r in rows)
        assert amounts == [100, 200]

    def test_cte_zero_rows(self) -> None:
        """CTE that returns no rows produces empty outer result."""
        conn = _make_conn()
        _setup_tables(conn)
        rows = conn.execute(
            "WITH empty AS (SELECT id FROM orders WHERE amount > 9999) "
            "SELECT id FROM empty"
        ).fetchall()
        assert rows == []

    def test_cte_with_subquery_in_where(self) -> None:
        """CTE body references outer real table; outer query uses CTE."""
        conn = _make_conn()
        _setup_tables(conn)
        rows = conn.execute(
            "WITH high_value AS ( "
            "    SELECT customer_id FROM orders WHERE amount > 100 "
            ") "
            "SELECT name FROM customers AS c "
            "INNER JOIN high_value AS hv ON c.id = hv.customer_id"
        ).fetchall()
        names = [r[0] for r in rows]
        assert names == ["Alice"]
