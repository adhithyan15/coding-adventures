"""LEFT OUTER JOIN integration tests — full SQL string through the pipeline.

These tests exercise the complete mini-sqlite stack:
    SQL string → parser → planner → optimizer → codegen → VM → rows

They verify that LEFT [OUTER] JOIN works correctly at the user-facing
SQL level, not just at the plan/IR level.
"""

from __future__ import annotations

import pytest

import mini_sqlite

# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture
def conn():
    """In-memory connection with customers + orders tables.

    customers:  id=1 Alice, id=2 Bob, id=3 Carol
    orders:     customer_id=1 alice_order, customer_id=2 bob_order

    LEFT JOIN customers → orders yields:
        (1, Alice, alice_order)
        (2, Bob,   bob_order)
        (3, Carol, NULL)
    """
    c = mini_sqlite.connect(":memory:")
    c.execute("""
        CREATE TABLE customers (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL
        )
    """)
    c.execute("""
        CREATE TABLE orders (
            order_id INTEGER PRIMARY KEY,
            customer_id INTEGER,
            product TEXT
        )
    """)
    c.executemany(
        "INSERT INTO customers (id, name) VALUES (?, ?)",
        [(1, "Alice"), (2, "Bob"), (3, "Carol")],
    )
    c.executemany(
        "INSERT INTO orders (order_id, customer_id, product) VALUES (?, ?, ?)",
        [(10, 1, "alice_order"), (20, 2, "bob_order")],
    )
    return c


# ---------------------------------------------------------------------------
# Basic LEFT OUTER JOIN
# ---------------------------------------------------------------------------


def test_left_outer_join_basic(conn) -> None:
    """Three left rows; two match, one (Carol) gets NULL for right columns."""
    rows = conn.execute("""
        SELECT c.name, o.product
        FROM customers AS c
        LEFT OUTER JOIN orders AS o ON c.id = o.customer_id
        ORDER BY c.name
    """).fetchall()

    assert len(rows) == 3
    by_name = {r[0]: r[1] for r in rows}
    assert by_name["Alice"] == "alice_order"
    assert by_name["Bob"] == "bob_order"
    assert by_name["Carol"] is None


def test_left_join_keyword_alone(conn) -> None:
    """LEFT JOIN (without OUTER) is identical to LEFT OUTER JOIN."""
    rows = conn.execute("""
        SELECT c.name, o.product
        FROM customers AS c
        LEFT JOIN orders AS o ON c.id = o.customer_id
        ORDER BY c.name
    """).fetchall()

    assert len(rows) == 3
    assert rows[2] == ("Carol", None)


def test_left_outer_join_no_matches(conn) -> None:
    """When the right table is empty every left row has NULL right cols."""
    conn.execute("DELETE FROM orders")
    rows = conn.execute("""
        SELECT c.name, o.product
        FROM customers AS c
        LEFT OUTER JOIN orders AS o ON c.id = o.customer_id
    """).fetchall()

    assert len(rows) == 3
    for row in rows:
        assert row[1] is None


def test_left_outer_join_all_match(conn) -> None:
    """When every left row matches, result is identical to INNER JOIN."""
    # Remove Carol so every remaining customer has an order
    conn.execute("DELETE FROM customers WHERE id = 3")
    rows = conn.execute("""
        SELECT c.name, o.product
        FROM customers AS c
        LEFT OUTER JOIN orders AS o ON c.id = o.customer_id
        ORDER BY c.name
    """).fetchall()

    assert rows == [("Alice", "alice_order"), ("Bob", "bob_order")]


def test_left_outer_join_multiple_right_matches(conn) -> None:
    """A left row with two matching right rows appears once per match."""
    conn.execute(
        "INSERT INTO orders (order_id, customer_id, product) VALUES (30, 1, 'alice_order_2')"
    )
    rows = conn.execute("""
        SELECT c.name, o.product
        FROM customers AS c
        LEFT OUTER JOIN orders AS o ON c.id = o.customer_id
        ORDER BY c.name, o.product
    """).fetchall()

    alice_products = [r[1] for r in rows if r[0] == "Alice"]
    assert sorted(alice_products) == ["alice_order", "alice_order_2"]

    carol_rows = [r for r in rows if r[0] == "Carol"]
    assert len(carol_rows) == 1
    assert carol_rows[0][1] is None


# ---------------------------------------------------------------------------
# SELECT * (ScanAllColumns) with LEFT OUTER JOIN
# ---------------------------------------------------------------------------


def test_left_outer_join_select_star(conn) -> None:
    """SELECT * returns all left-side rows including unmatched ones.

    Note: SELECT * over a JOIN currently expands only the first table's
    columns (a known limitation shared with INNER JOIN).  This test
    verifies the row count and that unmatched left rows still appear.
    """
    rows = conn.execute("""
        SELECT *
        FROM customers AS c
        LEFT OUTER JOIN orders AS o ON c.id = o.customer_id
    """).fetchall()

    # All three customers appear (unmatched Carol is not dropped).
    assert len(rows) == 3
    customer_ids = {r[0] for r in rows}
    assert customer_ids == {1, 2, 3}


# ---------------------------------------------------------------------------
# WHERE predicate applied after LEFT OUTER JOIN
# ---------------------------------------------------------------------------


def test_left_outer_join_where_null_right(conn) -> None:
    """WHERE product IS NULL selects only the unmatched left row (Carol)."""
    rows = conn.execute("""
        SELECT c.name
        FROM customers AS c
        LEFT OUTER JOIN orders AS o ON c.id = o.customer_id
        WHERE o.product IS NULL
    """).fetchall()

    assert rows == [("Carol",)]


def test_left_outer_join_where_not_null_right(conn) -> None:
    """WHERE product IS NOT NULL keeps only matched rows — equivalent to INNER JOIN."""
    rows = conn.execute("""
        SELECT c.name, o.product
        FROM customers AS c
        LEFT OUTER JOIN orders AS o ON c.id = o.customer_id
        WHERE o.product IS NOT NULL
        ORDER BY c.name
    """).fetchall()

    assert rows == [("Alice", "alice_order"), ("Bob", "bob_order")]


def test_left_outer_join_where_on_left_col(conn) -> None:
    """WHERE on left-side column filters correctly."""
    rows = conn.execute("""
        SELECT c.name, o.product
        FROM customers AS c
        LEFT OUTER JOIN orders AS o ON c.id = o.customer_id
        WHERE c.name = 'Alice'
    """).fetchall()

    assert rows == [("Alice", "alice_order")]


def test_left_outer_join_where_on_left_unmatched(conn) -> None:
    """WHERE on left-side column for an unmatched row still returns the row with NULL."""
    rows = conn.execute("""
        SELECT c.name, o.product
        FROM customers AS c
        LEFT OUTER JOIN orders AS o ON c.id = o.customer_id
        WHERE c.name = 'Carol'
    """).fetchall()

    assert rows == [("Carol", None)]


# ---------------------------------------------------------------------------
# Three-table chained LEFT OUTER JOIN
# ---------------------------------------------------------------------------


def test_left_outer_join_multiple_tables(conn) -> None:
    """customers LEFT JOIN orders LEFT JOIN shipments — three-way chain.

    Alice's order has no shipment → (Alice, alice_order, NULL)
    Bob's order ships → (Bob, bob_order, shipped)
    Carol has no order → (Carol, NULL, NULL)
    """
    conn.execute("""
        CREATE TABLE shipments (
            ship_id INTEGER PRIMARY KEY,
            order_id INTEGER,
            status TEXT
        )
    """)
    conn.execute(
        "INSERT INTO shipments (ship_id, order_id, status) VALUES (100, 20, 'shipped')"
    )

    rows = conn.execute("""
        SELECT c.name, o.product, s.status
        FROM customers AS c
        LEFT JOIN orders AS o ON c.id = o.customer_id
        LEFT JOIN shipments AS s ON o.order_id = s.order_id
        ORDER BY c.name
    """).fetchall()

    assert len(rows) == 3
    by_name = {r[0]: r for r in rows}
    assert by_name["Alice"] == ("Alice", "alice_order", None)
    assert by_name["Bob"] == ("Bob", "bob_order", "shipped")
    assert by_name["Carol"] == ("Carol", None, None)


# ---------------------------------------------------------------------------
# Aggregate + LEFT OUTER JOIN
# ---------------------------------------------------------------------------


def test_left_outer_join_with_count(conn) -> None:
    """COUNT of right-side column ignores NULLs (Carol contributes 0)."""
    rows = conn.execute("""
        SELECT c.name, COUNT(o.order_id) AS num_orders
        FROM customers AS c
        LEFT JOIN orders AS o ON c.id = o.customer_id
        GROUP BY c.name
        ORDER BY c.name
    """).fetchall()

    by_name = {r[0]: r[1] for r in rows}
    assert by_name["Alice"] == 1
    assert by_name["Bob"] == 1
    assert by_name["Carol"] == 0
