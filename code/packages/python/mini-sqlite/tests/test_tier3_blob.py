"""Tests for BLOB type — binary data via x'HEX' literals."""

import pytest

import mini_sqlite


@pytest.fixture()
def conn():
    c = mini_sqlite.connect(":memory:")
    c.execute("CREATE TABLE blobs (id INTEGER, data BLOB)")
    return c


def test_blob_insert_and_select(conn):
    """BLOB literal can be inserted and retrieved as bytes."""
    conn.execute("INSERT INTO blobs VALUES (1, x'DEADBEEF')")
    rows = conn.execute("SELECT data FROM blobs WHERE id = 1").fetchall()
    assert rows == [(bytes.fromhex("DEADBEEF"),)]


def test_blob_empty(conn):
    """Empty BLOB literal x'' produces an empty bytes object."""
    conn.execute("INSERT INTO blobs VALUES (2, x'')")
    rows = conn.execute("SELECT data FROM blobs WHERE id = 2").fetchall()
    assert rows == [(b"",)]


def test_blob_lowercase_hex(conn):
    """Lowercase hex digits in blob literal are accepted."""
    conn.execute("INSERT INTO blobs VALUES (3, x'cafebabe')")
    rows = conn.execute("SELECT data FROM blobs WHERE id = 3").fetchall()
    assert rows == [(bytes.fromhex("cafebabe"),)]


def test_blob_uppercase_x(conn):
    """X'' (uppercase X) is also valid blob syntax."""
    conn.execute("INSERT INTO blobs VALUES (4, X'FF00FF')")
    rows = conn.execute("SELECT data FROM blobs WHERE id = 4").fetchall()
    assert rows == [(bytes.fromhex("FF00FF"),)]


def test_blob_equality_comparison(conn):
    """BLOB values can be compared for equality in WHERE."""
    conn.execute("INSERT INTO blobs VALUES (5, x'AABB')")
    conn.execute("INSERT INTO blobs VALUES (6, x'CCDD')")
    rows = conn.execute(
        "SELECT id FROM blobs WHERE data = x'AABB'"
    ).fetchall()
    assert rows == [(5,)]


def test_blob_null(conn):
    """NULL BLOB column stores NULL."""
    conn.execute("INSERT INTO blobs VALUES (7, NULL)")
    rows = conn.execute("SELECT data FROM blobs WHERE id = 7").fetchall()
    assert rows == [(None,)]


def test_blob_single_byte(conn):
    """Single-byte BLOB literal."""
    conn.execute("INSERT INTO blobs VALUES (8, x'42')")
    rows = conn.execute("SELECT data FROM blobs WHERE id = 8").fetchall()
    assert rows == [(b"B",)]


def test_blob_in_select_from_table():
    """BLOB literal returned from a scan SELECT."""
    c = mini_sqlite.connect(":memory:")
    c.execute("CREATE TABLE t (id INTEGER, data BLOB)")
    c.execute("INSERT INTO t VALUES (1, x'0102030405')")
    rows = c.execute("SELECT data FROM t WHERE id = 1").fetchall()
    assert rows == [(bytes([1, 2, 3, 4, 5]),)]


def test_blob_roundtrip_multiple_rows(conn):
    """Multiple BLOBs round-trip correctly."""
    data = [
        (10, bytes.fromhex("00")),
        (11, bytes.fromhex("FF")),
        (12, bytes.fromhex("0A0B0C")),
    ]
    for row_id, value in data:
        hex_str = value.hex().upper()
        conn.execute(f"INSERT INTO blobs VALUES ({row_id}, x'{hex_str}')")
    rows = conn.execute("SELECT id, data FROM blobs ORDER BY id").fetchall()
    assert rows == data
