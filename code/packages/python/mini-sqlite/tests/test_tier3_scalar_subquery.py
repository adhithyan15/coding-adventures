"""Tests for scalar subquery support — (SELECT expr FROM ...) in expression position."""

import pytest
import mini_sqlite


@pytest.fixture()
def conn():
    c = mini_sqlite.connect(":memory:")
    c.execute("CREATE TABLE orders (id INTEGER, customer_id INTEGER, amount REAL)")
    c.execute("CREATE TABLE customers (id INTEGER, name TEXT)")
    c.execute("INSERT INTO customers VALUES (1, 'Alice')")
    c.execute("INSERT INTO customers VALUES (2, 'Bob')")
    c.execute("INSERT INTO orders VALUES (1, 1, 100.0)")
    c.execute("INSERT INTO orders VALUES (2, 1, 50.0)")
    c.execute("INSERT INTO orders VALUES (3, 2, 200.0)")
    return c


def test_scalar_subquery_in_select_list(conn):
    """Non-correlated scalar subquery in SELECT list returns same value for every row."""
    rows = conn.execute(
        "SELECT name, (SELECT COUNT(*) FROM orders) FROM customers ORDER BY name"
    ).fetchall()
    assert rows == [("Alice", 3), ("Bob", 3)]


def test_scalar_subquery_in_where(conn):
    """Scalar subquery in WHERE clause for filtering."""
    rows = conn.execute(
        "SELECT name FROM customers WHERE id = (SELECT customer_id FROM orders WHERE amount = 200.0)"
    ).fetchall()
    assert rows == [("Bob",)]


def test_scalar_subquery_returns_null_when_empty(conn):
    """Scalar subquery returns NULL when no rows match."""
    rows = conn.execute(
        "SELECT id, (SELECT name FROM customers WHERE name = 'Nobody') AS nm FROM orders WHERE id = 1"
    ).fetchall()
    assert rows == [(1, None)]


def test_scalar_subquery_aggregate(conn):
    """Scalar subquery with aggregate function returns a single row."""
    rows = conn.execute(
        "SELECT id, (SELECT SUM(amount) FROM orders) FROM customers WHERE id = 1"
    ).fetchall()
    assert rows == [(1, 350.0)]


def test_scalar_subquery_in_select_no_table(conn):
    """Scalar subquery used as a computed column alongside a real scan."""
    rows = conn.execute(
        "SELECT id, (SELECT COUNT(*) FROM customers) FROM orders WHERE id = 1"
    ).fetchall()
    assert rows == [(1, 2)]


def test_scalar_subquery_cardinality_error(conn):
    """Scalar subquery returning more than one row raises an error."""
    with pytest.raises(Exception):
        conn.execute("SELECT (SELECT id FROM customers)").fetchall()


def test_scalar_subquery_in_having(conn):
    """Scalar subquery in WHERE used to filter before grouping."""
    # Find customers whose first order was order id 1.
    rows = conn.execute(
        "SELECT customer_id, SUM(amount) FROM orders "
        "WHERE customer_id = (SELECT customer_id FROM orders WHERE id = 1) "
        "GROUP BY customer_id"
    ).fetchall()
    assert rows == [(1, 150.0)]


def test_scalar_subquery_in_computed_expression(conn):
    """Scalar subquery in WHERE — compare against a scalar aggregate."""
    rows = conn.execute(
        "SELECT id FROM orders WHERE amount = (SELECT MAX(amount) FROM orders)"
    ).fetchall()
    assert rows == [(3,)]
