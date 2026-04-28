"""
Phase 6: CREATE / DROP VIEW.

Tests are organised in three classes:

  TestViewGrammar       — grammar and adapter pipeline unit tests
  TestViewIntegration   — end-to-end SQL correctness tests via mini-sqlite
  TestViewErrors        — error handling tests (bad view names, IF EXISTS, etc.)
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
# TestViewGrammar — pipeline unit tests
# ---------------------------------------------------------------------------


class TestViewGrammar:
    """Verify CREATE/DROP VIEW syntax parses and the adapter builds the right AST."""

    def test_grammar_parses_create_view(self) -> None:
        """CREATE VIEW v AS SELECT … parses to a valid program ASTNode."""
        from sql_parser import parse_sql  # type: ignore[import]

        sql = "CREATE VIEW active AS SELECT id, name FROM customers WHERE region = 'east'"
        tree = parse_sql(sql)
        assert tree is not None
        assert tree.rule_name == "program"

    def test_grammar_parses_create_view_if_not_exists(self) -> None:
        """CREATE VIEW IF NOT EXISTS parses without error."""
        from sql_parser import parse_sql  # type: ignore[import]

        sql = "CREATE VIEW IF NOT EXISTS v AS SELECT 1 AS n FROM customers"
        tree = parse_sql(sql)
        assert tree is not None

    def test_grammar_parses_drop_view(self) -> None:
        """DROP VIEW v parses to a valid program ASTNode."""
        from sql_parser import parse_sql  # type: ignore[import]

        sql = "DROP VIEW active"
        tree = parse_sql(sql)
        assert tree is not None
        assert tree.rule_name == "program"

    def test_grammar_parses_drop_view_if_exists(self) -> None:
        """DROP VIEW IF EXISTS v parses without error."""
        from sql_parser import parse_sql  # type: ignore[import]

        sql = "DROP VIEW IF EXISTS v"
        tree = parse_sql(sql)
        assert tree is not None

    def test_adapter_produces_create_view_stmt(self) -> None:
        """Adapter converts CREATE VIEW into a CreateViewStmt with correct fields."""
        from sql_parser import parse_sql  # type: ignore[import]
        from sql_planner.ast import CreateViewStmt, SelectStmt  # type: ignore[import]

        from mini_sqlite.adapter import to_statement  # type: ignore[import]

        sql = "CREATE VIEW active AS SELECT id FROM customers WHERE region = 'east'"
        tree = parse_sql(sql)
        stmt = to_statement(tree)
        assert isinstance(stmt, CreateViewStmt)
        assert stmt.name == "active"
        assert isinstance(stmt.query, SelectStmt)
        assert not stmt.if_not_exists

    def test_adapter_produces_create_view_stmt_if_not_exists(self) -> None:
        """Adapter sets if_not_exists=True when IF NOT EXISTS is present."""
        from sql_parser import parse_sql  # type: ignore[import]
        from sql_planner.ast import CreateViewStmt  # type: ignore[import]

        from mini_sqlite.adapter import to_statement  # type: ignore[import]

        sql = "CREATE VIEW IF NOT EXISTS v AS SELECT 1 AS n FROM customers"
        tree = parse_sql(sql)
        stmt = to_statement(tree)
        assert isinstance(stmt, CreateViewStmt)
        assert stmt.if_not_exists

    def test_adapter_produces_drop_view_stmt(self) -> None:
        """Adapter converts DROP VIEW into a DropViewStmt with correct fields."""
        from sql_parser import parse_sql  # type: ignore[import]
        from sql_planner.ast import DropViewStmt  # type: ignore[import]

        from mini_sqlite.adapter import to_statement  # type: ignore[import]

        sql = "DROP VIEW active"
        tree = parse_sql(sql)
        stmt = to_statement(tree)
        assert isinstance(stmt, DropViewStmt)
        assert stmt.name == "active"
        assert not stmt.if_exists

    def test_adapter_produces_drop_view_stmt_if_exists(self) -> None:
        """Adapter sets if_exists=True when IF EXISTS is present."""
        from sql_parser import parse_sql  # type: ignore[import]
        from sql_planner.ast import DropViewStmt  # type: ignore[import]

        from mini_sqlite.adapter import to_statement  # type: ignore[import]

        sql = "DROP VIEW IF EXISTS v"
        tree = parse_sql(sql)
        stmt = to_statement(tree)
        assert isinstance(stmt, DropViewStmt)
        assert stmt.if_exists

    def test_adapter_expands_view_in_from(self) -> None:
        """Adapter expands a view name in FROM to a DerivedTableRef when view_defs supplied."""
        from sql_parser import parse_sql  # type: ignore[import]
        from sql_planner.ast import DerivedTableRef, SelectStmt  # type: ignore[import]

        from mini_sqlite.adapter import to_statement  # type: ignore[import]

        # First parse the view body.
        view_body_sql = "SELECT id FROM customers WHERE region = 'east'"
        view_tree = parse_sql(view_body_sql)
        view_stmt = to_statement(view_tree)
        assert isinstance(view_stmt, SelectStmt)

        # Now parse a SELECT that references the view, supplying view_defs.
        query_sql = "SELECT id FROM active"
        query_tree = parse_sql(query_sql)
        result = to_statement(query_tree, view_defs={"active": view_stmt})
        assert isinstance(result, SelectStmt)
        assert isinstance(result.from_, DerivedTableRef)
        assert result.from_.alias == "active"

    def test_adapter_view_alias_overrides_default(self) -> None:
        """When SELECT uses AS alias on a view, the alias is preserved."""
        from sql_parser import parse_sql  # type: ignore[import]
        from sql_planner.ast import DerivedTableRef, SelectStmt  # type: ignore[import]

        from mini_sqlite.adapter import to_statement  # type: ignore[import]

        view_body_sql = "SELECT id FROM customers"
        view_tree = parse_sql(view_body_sql)
        view_stmt = to_statement(view_tree)
        assert isinstance(view_stmt, SelectStmt)

        query_sql = "SELECT a.id FROM active AS a"
        query_tree = parse_sql(query_sql)
        result = to_statement(query_tree, view_defs={"active": view_stmt})
        assert isinstance(result, SelectStmt)
        assert isinstance(result.from_, DerivedTableRef)
        assert result.from_.alias == "a"


# ---------------------------------------------------------------------------
# TestViewIntegration — end-to-end SQL correctness tests
# ---------------------------------------------------------------------------


class TestViewIntegration:
    """Full pipeline: CREATE VIEW → SELECT … FROM view → correct rows."""

    def test_create_and_select_from_view(self) -> None:
        """Simple view returns the same rows as the underlying SELECT."""
        conn = _make_conn()
        _setup_tables(conn)
        conn.execute(
            "CREATE VIEW east_customers AS "
            "SELECT id, name FROM customers WHERE region = 'east'"
        )
        rows = conn.execute("SELECT id, name FROM east_customers").fetchall()
        assert sorted(rows) == [(1, "Alice"), (3, "Carol")]

    def test_view_with_alias(self) -> None:
        """View can be queried with an explicit alias."""
        conn = _make_conn()
        _setup_tables(conn)
        conn.execute(
            "CREATE VIEW east_customers AS "
            "SELECT id, name FROM customers WHERE region = 'east'"
        )
        rows = conn.execute("SELECT ec.id FROM east_customers AS ec").fetchall()
        assert sorted(rows) == [(1,), (3,)]

    def test_view_used_in_join(self) -> None:
        """View can appear in a JOIN clause."""
        conn = _make_conn()
        _setup_tables(conn)
        conn.execute(
            "CREATE VIEW east_customers AS "
            "SELECT id FROM customers WHERE region = 'east'"
        )
        rows = conn.execute(
            "SELECT o.id FROM orders AS o "
            "INNER JOIN east_customers AS ec ON o.customer_id = ec.id"
        ).fetchall()
        # Alice (id=1) has orders 1 and 2; Carol (id=3) has order 4.
        assert sorted(rows) == [(1,), (2,), (4,)]

    def test_view_with_aggregation(self) -> None:
        """View body can include aggregation; outer SELECT queries the view normally."""
        conn = _make_conn()
        _setup_tables(conn)
        conn.execute(
            "CREATE VIEW customer_totals AS "
            "SELECT customer_id, SUM(amount) AS total FROM orders GROUP BY customer_id"
        )
        rows = conn.execute("SELECT customer_id, total FROM customer_totals").fetchall()
        result = {cid: total for cid, total in rows}
        assert result[1] == 300   # Alice: 100 + 200
        assert result[2] == 50    # Bob
        assert result[3] == 75    # Carol

    def test_drop_view_removes_it(self) -> None:
        """After DROP VIEW the name is no longer accessible."""
        from mini_sqlite.errors import OperationalError  # type: ignore[import]

        conn = _make_conn()
        _setup_tables(conn)
        conn.execute("CREATE VIEW v AS SELECT id FROM customers")
        conn.execute("DROP VIEW v")
        # After dropping, SELECT from the view should fail with an unknown-table error.
        with pytest.raises(OperationalError):
            conn.execute("SELECT id FROM v").fetchall()

    def test_create_view_if_not_exists_is_idempotent(self) -> None:
        """CREATE VIEW IF NOT EXISTS does not raise when the view already exists."""
        conn = _make_conn()
        _setup_tables(conn)
        conn.execute(
            "CREATE VIEW v AS SELECT id FROM customers WHERE region = 'east'"
        )
        # Second CREATE with IF NOT EXISTS should not raise.
        conn.execute(
            "CREATE VIEW IF NOT EXISTS v AS SELECT id FROM customers WHERE region = 'west'"
        )
        # The original definition (east) must be preserved.
        rows = conn.execute("SELECT id FROM v").fetchall()
        assert sorted(rows) == [(1,), (3,)]

    def test_drop_view_if_exists_no_error_when_missing(self) -> None:
        """DROP VIEW IF EXISTS on a nonexistent view is a silent no-op."""
        conn = _make_conn()
        conn.execute("DROP VIEW IF EXISTS nonexistent")  # must not raise

    def test_view_persists_across_multiple_queries(self) -> None:
        """A created view is accessible across separate execute() calls on the same connection."""
        conn = _make_conn()
        _setup_tables(conn)
        conn.execute("CREATE VIEW east_cust AS SELECT id FROM customers WHERE region = 'east'")
        # Separate execute calls on the same connection should still see the view.
        r1 = conn.execute("SELECT id FROM east_cust").fetchall()
        r2 = conn.execute("SELECT id FROM east_cust WHERE id > 1").fetchall()
        assert sorted(r1) == [(1,), (3,)]
        assert sorted(r2) == [(3,)]

    def test_view_with_order_by(self) -> None:
        """View body can contain ORDER BY and the outer query respects it."""
        conn = _make_conn()
        _setup_tables(conn)
        conn.execute(
            "CREATE VIEW ordered_cust AS SELECT id, name FROM customers ORDER BY name ASC"
        )
        rows = conn.execute("SELECT name FROM ordered_cust").fetchall()
        names = [r[0] for r in rows]
        assert names == sorted(names)

    def test_view_in_where_subquery_expands_correctly(self) -> None:
        """View referenced in a JOIN with a WHERE predicate on the outer query works."""
        conn = _make_conn()
        _setup_tables(conn)
        conn.execute(
            "CREATE VIEW big_orders AS "
            "SELECT id, customer_id FROM orders WHERE amount > 60"
        )
        rows = conn.execute("SELECT id FROM big_orders WHERE customer_id = 1").fetchall()
        assert sorted(rows) == [(1,), (2,)]


# ---------------------------------------------------------------------------
# TestViewErrors — error handling
# ---------------------------------------------------------------------------


class TestViewErrors:
    """Verify that view-related errors are raised with helpful messages."""

    def test_drop_nonexistent_view_raises(self) -> None:
        """DROP VIEW on an unknown view (without IF EXISTS) must raise."""
        conn = _make_conn()
        with pytest.raises(Exception, match="no such view"):
            conn.execute("DROP VIEW nonexistent")

    def test_create_view_duplicate_raises(self) -> None:
        """CREATE VIEW (without IF NOT EXISTS) on an existing view name must raise."""
        from mini_sqlite.errors import ProgrammingError  # type: ignore[import]

        conn = _make_conn()
        conn.execute("CREATE TABLE t (id INTEGER)")
        conn.execute("CREATE VIEW v AS SELECT id FROM t")
        with pytest.raises(ProgrammingError, match="already exists"):
            conn.execute("CREATE VIEW v AS SELECT id FROM t")

    def test_view_not_visible_after_drop(self) -> None:
        """A view dropped within the same connection is not accessible afterwards."""
        from mini_sqlite.errors import OperationalError  # type: ignore[import]

        conn = _make_conn()
        conn.execute("CREATE TABLE t (id INTEGER)")
        conn.execute("INSERT INTO t VALUES (1)")
        conn.execute("CREATE VIEW v AS SELECT id FROM t")
        assert conn.execute("SELECT id FROM v").fetchall() == [(1,)]
        conn.execute("DROP VIEW v")
        with pytest.raises(OperationalError):
            conn.execute("SELECT id FROM v").fetchall()
