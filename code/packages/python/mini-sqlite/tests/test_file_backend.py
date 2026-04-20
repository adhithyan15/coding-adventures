"""
Tests for file-backed connect() (phase 8).

Two families of tests:

1. **File-backend functional tests** — exercise the same SQL operations
   that the in-memory tests cover, but against a real ``.db`` file via
   ``mini_sqlite.connect("path.db")``.  Persistence, transactions, DDL,
   DML, and constraints are all verified end-to-end.

2. **Byte-compatibility oracle tests** — the "oracle" is Python's built-in
   ``sqlite3`` module, which ships with CPython and reads/writes the real
   SQLite file format.  Tests in this family:

   * Write via mini_sqlite, read via ``sqlite3``.
   * Write via ``sqlite3``, read via mini_sqlite.

   If both directions pass, the file format is byte-compatible: the same
   ``.db`` file can be opened by either implementation.

Why a separate test file?
--------------------------
``test_integration.py`` covers the in-memory backend end-to-end.  We keep
file-backend tests here so the oracle tests (which depend on Python's
``sqlite3`` stdlib) are isolated and easy to identify.
"""

from __future__ import annotations

import sqlite3
from pathlib import Path

import pytest

import mini_sqlite

# ---------------------------------------------------------------------------
# File-backend functional tests
# ---------------------------------------------------------------------------


def test_file_connect_creates_database(tmp_path: Path) -> None:
    """connect(path) creates the .db file on disk."""
    path = str(tmp_path / "new.db")
    conn = mini_sqlite.connect(path)
    conn.close()
    assert (tmp_path / "new.db").exists()


def test_file_connect_reopen(tmp_path: Path) -> None:
    """A second connect() call on the same path opens the existing file."""
    path = str(tmp_path / "reopen.db")
    with mini_sqlite.connect(path) as conn:
        conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)")
        conn.execute("INSERT INTO t VALUES (1, 'hello')")

    # Re-open and read back.
    with mini_sqlite.connect(path) as conn:
        rows = conn.execute("SELECT id, val FROM t").fetchall()

    assert rows == [(1, "hello")]


def test_file_create_table_and_insert(tmp_path: Path) -> None:
    """Full DDL + DML round-trip through the file backend."""
    path = str(tmp_path / "dml.db")
    with mini_sqlite.connect(path) as conn:
        conn.execute(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, age INTEGER)"
        )
        conn.executemany(
            "INSERT INTO users VALUES (?, ?, ?)",
            [(1, "Alice", 30), (2, "Bob", 25), (3, "Carol", 35)],
        )

    with mini_sqlite.connect(path) as conn:
        rows = conn.execute("SELECT id, name, age FROM users ORDER BY id").fetchall()

    assert rows == [(1, "Alice", 30), (2, "Bob", 25), (3, "Carol", 35)]


def test_file_select_where(tmp_path: Path) -> None:
    """WHERE filtering works against file-backed tables."""
    path = str(tmp_path / "where.db")
    with mini_sqlite.connect(path) as conn:
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.executemany("INSERT INTO t VALUES (?)", [(i,) for i in range(10)])

    with mini_sqlite.connect(path) as conn:
        rows = conn.execute("SELECT x FROM t WHERE x > 5 ORDER BY x").fetchall()

    assert rows == [(6,), (7,), (8,), (9,)]


def test_file_update(tmp_path: Path) -> None:
    """UPDATE is visible after commit and re-open."""
    path = str(tmp_path / "update.db")
    with mini_sqlite.connect(path) as conn:
        conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
        conn.execute("INSERT INTO t VALUES (1, 'old')")

    with mini_sqlite.connect(path) as conn:
        conn.execute("UPDATE t SET v = 'new' WHERE id = 1")

    with mini_sqlite.connect(path) as conn:
        row = conn.execute("SELECT v FROM t WHERE id = 1").fetchone()

    assert row == ("new",)


def test_file_delete(tmp_path: Path) -> None:
    """DELETE is visible after commit and re-open."""
    path = str(tmp_path / "delete.db")
    with mini_sqlite.connect(path) as conn:
        conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY)")
        conn.executemany("INSERT INTO t VALUES (?)", [(1,), (2,), (3,)])

    with mini_sqlite.connect(path) as conn:
        conn.execute("DELETE FROM t WHERE id = 2")

    with mini_sqlite.connect(path) as conn:
        rows = conn.execute("SELECT id FROM t ORDER BY id").fetchall()

    assert rows == [(1,), (3,)]


def test_file_drop_table(tmp_path: Path) -> None:
    """DROP TABLE removes the table; subsequent SELECT raises ProgrammingError."""
    path = str(tmp_path / "drop.db")
    with mini_sqlite.connect(path) as conn:
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (1)")

    with mini_sqlite.connect(path) as conn:
        conn.execute("DROP TABLE t")

    with mini_sqlite.connect(path) as conn, pytest.raises(mini_sqlite.OperationalError):
        conn.execute("SELECT * FROM t")


def test_file_transaction_commit(tmp_path: Path) -> None:
    """Explicit commit makes inserts durable."""
    path = str(tmp_path / "txn_commit.db")
    conn = mini_sqlite.connect(path)
    conn.execute("CREATE TABLE t (x INTEGER)")
    conn.execute("INSERT INTO t VALUES (42)")
    conn.commit()
    conn.close()

    with mini_sqlite.connect(path) as conn2:
        row = conn2.execute("SELECT x FROM t").fetchone()
    assert row == (42,)


def test_file_transaction_rollback(tmp_path: Path) -> None:
    """Explicit rollback discards uncommitted inserts."""
    path = str(tmp_path / "txn_rollback.db")
    # Seed the table.
    with mini_sqlite.connect(path) as conn:
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (1)")

    conn = mini_sqlite.connect(path)
    conn.execute("INSERT INTO t VALUES (99)")
    conn.rollback()
    conn.close()

    with mini_sqlite.connect(path) as conn2:
        rows = conn2.execute("SELECT x FROM t ORDER BY x").fetchall()
    assert rows == [(1,)]


def test_file_context_manager_commits_on_success(tmp_path: Path) -> None:
    """``with connect(path) as conn:`` commits on clean exit."""
    path = str(tmp_path / "ctx_commit.db")
    with mini_sqlite.connect(path) as conn:
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (7)")
    # The context manager should have committed.
    with mini_sqlite.connect(path) as conn2:
        row = conn2.execute("SELECT x FROM t").fetchone()
    assert row == (7,)


def test_file_context_manager_rollback_on_exception(tmp_path: Path) -> None:
    """``with connect(path) as conn:`` rolls back when an exception escapes."""
    path = str(tmp_path / "ctx_rollback.db")
    # Seed.
    with mini_sqlite.connect(path) as conn:
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (1)")

    try:
        with mini_sqlite.connect(path) as conn:
            conn.execute("INSERT INTO t VALUES (99)")
            raise RuntimeError("abort!")
    except RuntimeError:
        pass

    with mini_sqlite.connect(path) as conn2:
        rows = conn2.execute("SELECT x FROM t ORDER BY x").fetchall()
    assert rows == [(1,)]


def test_file_null_values(tmp_path: Path) -> None:
    """NULL values survive a write-read round-trip to disk."""
    path = str(tmp_path / "null.db")
    with mini_sqlite.connect(path) as conn:
        conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
        conn.execute("INSERT INTO t VALUES (1, NULL)")

    with mini_sqlite.connect(path) as conn:
        row = conn.execute("SELECT id, v FROM t").fetchone()

    assert row == (1, None)


def test_file_large_table(tmp_path: Path) -> None:
    """500 rows survive a write-read cycle (exercises B-tree splits)."""
    path = str(tmp_path / "large.db")
    n = 500
    with mini_sqlite.connect(path) as conn:
        conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, label TEXT)")
        conn.executemany(
            "INSERT INTO t VALUES (?, ?)",
            [(i, f"row-{i:05d}") for i in range(1, n + 1)],
        )

    with mini_sqlite.connect(path) as conn:
        rows = conn.execute("SELECT id FROM t ORDER BY id").fetchall()

    assert [r[0] for r in rows] == list(range(1, n + 1))


def test_file_multiple_tables(tmp_path: Path) -> None:
    """Multiple tables can be created and queried independently."""
    path = str(tmp_path / "multi.db")
    with mini_sqlite.connect(path) as conn:
        conn.execute("CREATE TABLE a (x INTEGER)")
        conn.execute("CREATE TABLE b (y TEXT)")
        conn.execute("INSERT INTO a VALUES (1)")
        conn.execute("INSERT INTO b VALUES ('hello')")

    with mini_sqlite.connect(path) as conn:
        ax = conn.execute("SELECT x FROM a").fetchone()
        by = conn.execute("SELECT y FROM b").fetchone()

    assert ax == (1,)
    assert by == ("hello",)


# ---------------------------------------------------------------------------
# Byte-compatibility oracle tests
# ---------------------------------------------------------------------------
# These tests use Python's built-in sqlite3 module as the oracle.
# They verify that files written by mini_sqlite are readable by sqlite3,
# and vice versa.
# ---------------------------------------------------------------------------


def test_oracle_mini_sqlite_writes_sqlite3_reads(tmp_path: Path) -> None:
    """Write via mini_sqlite; read back via Python's stdlib sqlite3.

    If sqlite3 can open the file and see the correct rows, the on-disk
    format is byte-compatible.
    """
    path = str(tmp_path / "oracle_write.db")

    # Write with mini_sqlite.
    with mini_sqlite.connect(path) as conn:
        conn.execute(
            "CREATE TABLE people (id INTEGER PRIMARY KEY, name TEXT NOT NULL, age INTEGER)"
        )
        conn.executemany(
            "INSERT INTO people VALUES (?, ?, ?)",
            [(1, "Alice", 30), (2, "Bob", 25), (3, "Carol", 35)],
        )

    # Read with stdlib sqlite3.
    with sqlite3.connect(path) as db:
        rows = db.execute("SELECT id, name, age FROM people ORDER BY id").fetchall()

    assert rows == [(1, "Alice", 30), (2, "Bob", 25), (3, "Carol", 35)]


def test_oracle_sqlite3_writes_mini_sqlite_reads(tmp_path: Path) -> None:
    """Write via Python's stdlib sqlite3; read back via mini_sqlite.

    Verifies that mini_sqlite can correctly parse files produced by the
    reference implementation.
    """
    path = str(tmp_path / "oracle_read.db")

    # Write with stdlib sqlite3.
    with sqlite3.connect(path) as db:
        db.execute(
            "CREATE TABLE items (id INTEGER PRIMARY KEY, label TEXT, score REAL)"
        )
        db.executemany(
            "INSERT INTO items VALUES (?, ?, ?)",
            [(1, "alpha", 1.5), (2, "beta", 2.75), (3, "gamma", 0.0)],
        )

    # Read with mini_sqlite.
    with mini_sqlite.connect(path) as conn:
        rows = conn.execute("SELECT id, label, score FROM items ORDER BY id").fetchall()

    assert rows == [(1, "alpha", 1.5), (2, "beta", 2.75), (3, "gamma", 0.0)]


def test_oracle_null_roundtrip(tmp_path: Path) -> None:
    """NULL values written by mini_sqlite are read as None by sqlite3."""
    path = str(tmp_path / "null_oracle.db")

    with mini_sqlite.connect(path) as conn:
        conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
        conn.execute("INSERT INTO t VALUES (1, NULL)")
        conn.execute("INSERT INTO t VALUES (2, 'present')")

    with sqlite3.connect(path) as db:
        rows = db.execute("SELECT id, v FROM t ORDER BY id").fetchall()

    assert rows == [(1, None), (2, "present")]


def test_oracle_sqlite3_null_read_by_mini_sqlite(tmp_path: Path) -> None:
    """NULL values written by sqlite3 are read as None by mini_sqlite."""
    path = str(tmp_path / "null_oracle_rev.db")

    with sqlite3.connect(path) as db:
        db.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
        db.execute("INSERT INTO t VALUES (1, NULL)")
        db.execute("INSERT INTO t VALUES (2, 'here')")

    with mini_sqlite.connect(path) as conn:
        rows = conn.execute("SELECT id, v FROM t ORDER BY id").fetchall()

    assert rows == [(1, None), (2, "here")]


def test_oracle_integer_types(tmp_path: Path) -> None:
    """Integer values across the full SQLite type range round-trip correctly."""
    path = str(tmp_path / "ints.db")
    values = [0, 1, 127, 128, 255, 256, 32767, 32768, 2**31 - 1, 2**31, 2**63 - 1]

    with mini_sqlite.connect(path) as conn:
        conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, n INTEGER)")
        conn.executemany(
            "INSERT INTO t VALUES (?, ?)",
            list(enumerate(values, start=1)),
        )

    # Verify via stdlib sqlite3.
    with sqlite3.connect(path) as db:
        rows = db.execute("SELECT n FROM t ORDER BY id").fetchall()

    assert [r[0] for r in rows] == values


def test_oracle_text_with_special_characters(tmp_path: Path) -> None:
    """Text with quotes, Unicode, and newlines survives the round-trip."""
    path = str(tmp_path / "text.db")
    samples = [
        "hello world",
        "it's a test",
        "line1\nline2",
        "tab\there",
        "café résumé",
        "日本語",
        "emoji 🎉",
    ]

    with mini_sqlite.connect(path) as conn:
        conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, s TEXT)")
        conn.executemany(
            "INSERT INTO t VALUES (?, ?)",
            list(enumerate(samples, start=1)),
        )

    with sqlite3.connect(path) as db:
        rows = db.execute("SELECT s FROM t ORDER BY id").fetchall()

    assert [r[0] for r in rows] == samples


def test_oracle_schema_visible_in_sqlite3(tmp_path: Path) -> None:
    """The sqlite_schema table written by mini_sqlite is visible to sqlite3."""
    path = str(tmp_path / "schema.db")

    with mini_sqlite.connect(path) as conn:
        conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)")
        conn.execute("CREATE TABLE orders (id INTEGER PRIMARY KEY, user_id INTEGER)")

    with sqlite3.connect(path) as db:
        names = {
            row[0]
            for row in db.execute(
                "SELECT name FROM sqlite_schema WHERE type='table'"
            ).fetchall()
        }

    assert "users" in names
    assert "orders" in names


def test_oracle_append_then_read_all(tmp_path: Path) -> None:
    """Write in two separate mini_sqlite sessions; sqlite3 sees all rows."""
    path = str(tmp_path / "append.db")

    with mini_sqlite.connect(path) as conn:
        conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
        conn.execute("INSERT INTO t VALUES (1, 'first')")

    with mini_sqlite.connect(path) as conn:
        conn.execute("INSERT INTO t VALUES (2, 'second')")

    with sqlite3.connect(path) as db:
        rows = db.execute("SELECT id, v FROM t ORDER BY id").fetchall()

    assert rows == [(1, "first"), (2, "second")]
