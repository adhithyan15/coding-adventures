"""Tests for PRAGMA statement support."""

import pytest
import mini_sqlite


@pytest.fixture()
def conn():
    c = mini_sqlite.connect(":memory:")
    c.execute(
        "CREATE TABLE products ("
        "  id INTEGER PRIMARY KEY, "
        "  name TEXT NOT NULL, "
        "  category TEXT, "
        "  price REAL"
        ")"
    )
    c.execute("CREATE INDEX idx_category ON products (category)")
    c.execute("CREATE INDEX idx_name ON products (name)")
    c.execute("CREATE TABLE orders (id INTEGER, product_id INTEGER)")
    return c


def test_pragma_table_info(conn):
    """PRAGMA table_info returns column metadata for a table."""
    rows = conn.execute("PRAGMA table_info(products)").fetchall()
    # Each row: (cid, name, type, notnull, dflt_value, pk)
    assert len(rows) == 4
    names = [r[1] for r in rows]
    assert names == ["id", "name", "category", "price"]
    types = [r[2] for r in rows]
    assert types == ["INTEGER", "TEXT", "TEXT", "REAL"]
    # notnull flag
    notnull = {r[1]: r[3] for r in rows}
    assert notnull["name"] == 1
    assert notnull["category"] == 0
    # pk flag
    pk = {r[1]: r[5] for r in rows}
    assert pk["id"] == 1
    assert pk["name"] == 0


def test_pragma_table_info_unknown_table(conn):
    """PRAGMA table_info on a non-existent table returns empty result."""
    rows = conn.execute("PRAGMA table_info(nonexistent)").fetchall()
    assert rows == []


def test_pragma_index_list(conn):
    """PRAGMA index_list returns indexes for a table."""
    rows = conn.execute("PRAGMA index_list(products)").fetchall()
    # Each row: (seq, name, unique, origin, partial)
    index_names = {r[1] for r in rows}
    assert "idx_category" in index_names
    assert "idx_name" in index_names


def test_pragma_index_list_no_indexes(conn):
    """PRAGMA index_list on a table with no indexes returns empty result."""
    rows = conn.execute("PRAGMA index_list(orders)").fetchall()
    assert rows == []


def test_pragma_table_list(conn):
    """PRAGMA table_list returns all tables in the database."""
    rows = conn.execute("PRAGMA table_list").fetchall()
    table_names = {r[1] for r in rows}
    assert "products" in table_names
    assert "orders" in table_names


def test_pragma_foreign_key_list_empty(conn):
    """PRAGMA foreign_key_list on a table with no FKs returns empty result."""
    rows = conn.execute("PRAGMA foreign_key_list(products)").fetchall()
    assert rows == []


def test_pragma_foreign_key_list(conn):
    """PRAGMA foreign_key_list returns foreign key constraints."""
    c = mini_sqlite.connect(":memory:")
    c.execute("CREATE TABLE parent (id INTEGER PRIMARY KEY)")
    c.execute(
        "CREATE TABLE child ("
        "  id INTEGER PRIMARY KEY, "
        "  parent_id INTEGER REFERENCES parent(id)"
        ")"
    )
    rows = c.execute("PRAGMA foreign_key_list(child)").fetchall()
    assert len(rows) == 1
    # row: (id, seq, table, from, to, ...)
    assert rows[0][2] == "parent"
    assert rows[0][3] == "parent_id"


def test_pragma_case_insensitive(conn):
    """PRAGMA keyword is case-insensitive."""
    rows1 = conn.execute("pragma table_info(products)").fetchall()
    rows2 = conn.execute("PRAGMA TABLE_INFO(products)").fetchall()
    assert len(rows1) == len(rows2) == 4


def test_pragma_table_info_order_by_cid(conn):
    """Column order (cid) matches CREATE TABLE column order."""
    rows = conn.execute("PRAGMA table_info(products)").fetchall()
    cids = [r[0] for r in rows]
    assert cids == list(range(len(rows)))
