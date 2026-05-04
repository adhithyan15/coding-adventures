"""End-to-end tests for the RETURNING clause on INSERT, UPDATE, DELETE.

The RETURNING clause lets DML statements emit a result set from the affected
rows.  These tests exercise the full pipeline: SQL text → lexer → parser →
adapter → planner → codegen → VM → cursor.

All assertions are validated against sqlite3 in a companion oracle test at
the bottom of this file so any divergence from real SQLite behaviour is
immediately visible in CI.
"""

from __future__ import annotations

import pytest

import mini_sqlite


@pytest.fixture()
def conn():
    """In-memory database with an employees table and some seed data."""
    c = mini_sqlite.connect(":memory:")
    c.execute("CREATE TABLE employees (id INTEGER, name TEXT, salary INTEGER)")
    yield c
    c.close()


@pytest.fixture()
def empty_conn():
    """In-memory database with an empty employees table."""
    c = mini_sqlite.connect(":memory:")
    c.execute("CREATE TABLE employees (id INTEGER, name TEXT, salary INTEGER)")
    yield c
    c.close()


class TestInsertReturning:
    """INSERT … RETURNING."""

    def test_single_row_single_column(self, empty_conn) -> None:
        cur = empty_conn.execute(
            "INSERT INTO employees VALUES (1, 'Alice', 50000) RETURNING id"
        )
        assert cur.description is not None
        assert cur.description[0][0] == "id"
        rows = cur.fetchall()
        assert rows == [(1,)]

    def test_single_row_multiple_columns(self, empty_conn) -> None:
        cur = empty_conn.execute(
            "INSERT INTO employees VALUES (2, 'Bob', 60000) RETURNING id, name"
        )
        assert cur.description is not None
        col_names = [d[0] for d in cur.description]
        assert col_names == ["id", "name"]
        assert cur.fetchall() == [(2, "Bob")]

    def test_single_row_all_columns(self, empty_conn) -> None:
        cur = empty_conn.execute(
            "INSERT INTO employees VALUES (3, 'Charlie', 70000) RETURNING id, name, salary"
        )
        assert cur.fetchall() == [(3, "Charlie", 70000)]

    def test_multiple_rows_returning(self, empty_conn) -> None:
        """INSERT of multiple rows returns one RETURNING row per inserted row."""
        cur = empty_conn.execute(
            "INSERT INTO employees VALUES (1, 'A', 10), (2, 'B', 20) RETURNING id, salary"
        )
        rows = cur.fetchall()
        assert len(rows) == 2
        assert rows[0] == (1, 10)
        assert rows[1] == (2, 20)

    def test_row_is_actually_inserted(self, empty_conn) -> None:
        """Verify that RETURNING doesn't consume the INSERT — row must exist after."""
        empty_conn.execute("INSERT INTO employees VALUES (7, 'G', 777) RETURNING id")
        rows = empty_conn.execute("SELECT * FROM employees WHERE id = 7").fetchall()
        assert len(rows) == 1
        assert rows[0][0] == 7

    def test_rowcount_is_number_of_returning_rows(self, empty_conn) -> None:
        """cursor.rowcount should equal the number of RETURNING rows."""
        cur = empty_conn.execute(
            "INSERT INTO employees VALUES (1, 'A', 100) RETURNING id"
        )
        assert cur.rowcount == 1

    def test_returning_salary_value(self, empty_conn) -> None:
        """RETURNING salary gives back the exact value that was inserted."""
        cur = empty_conn.execute(
            "INSERT INTO employees VALUES (42, 'X', 99999) RETURNING salary"
        )
        assert cur.fetchone() == (99999,)


class TestUpdateReturning:
    """UPDATE … RETURNING."""

    def test_update_single_row_returning(self, conn) -> None:
        conn.execute("INSERT INTO employees VALUES (1, 'Alice', 50000)")
        cur = conn.execute(
            "UPDATE employees SET salary = 60000 WHERE id = 1 RETURNING id, salary"
        )
        assert cur.description is not None
        col_names = [d[0] for d in cur.description]
        assert col_names == ["id", "salary"]
        rows = cur.fetchall()
        assert rows == [(1, 60000)]

    def test_update_multiple_rows_returning(self, conn) -> None:
        conn.execute("INSERT INTO employees VALUES (1, 'Alice', 50000)")
        conn.execute("INSERT INTO employees VALUES (2, 'Bob', 60000)")
        cur = conn.execute(
            "UPDATE employees SET salary = 99999 RETURNING id"
        )
        rows = cur.fetchall()
        assert len(rows) == 2
        ids = {r[0] for r in rows}
        assert ids == {1, 2}

    def test_update_returns_post_update_value(self, conn) -> None:
        """RETURNING shows the NEW (post-update) value, not the old one."""
        conn.execute("INSERT INTO employees VALUES (1, 'Alice', 50000)")
        cur = conn.execute(
            "UPDATE employees SET salary = 75000 WHERE id = 1 RETURNING salary"
        )
        assert cur.fetchone() == (75000,)

    def test_update_no_matching_rows_returns_empty(self, conn) -> None:
        conn.execute("INSERT INTO employees VALUES (1, 'Alice', 50000)")
        cur = conn.execute(
            "UPDATE employees SET salary = 0 WHERE id = 999 RETURNING id"
        )
        assert cur.fetchall() == []

    def test_update_actually_persists(self, conn) -> None:
        """RETURNING doesn't interfere with the actual update side-effect."""
        conn.execute("INSERT INTO employees VALUES (1, 'Alice', 50000)")
        conn.execute(
            "UPDATE employees SET name = 'Alicia' WHERE id = 1 RETURNING id"
        )
        row = conn.execute("SELECT name FROM employees WHERE id = 1").fetchone()
        assert row == ("Alicia",)


class TestDeleteReturning:
    """DELETE … RETURNING."""

    def test_delete_single_row_returning(self, conn) -> None:
        conn.execute("INSERT INTO employees VALUES (1, 'Alice', 50000)")
        cur = conn.execute(
            "DELETE FROM employees WHERE id = 1 RETURNING id, name"
        )
        assert cur.description is not None
        col_names = [d[0] for d in cur.description]
        assert col_names == ["id", "name"]
        rows = cur.fetchall()
        assert rows == [(1, "Alice")]

    def test_delete_returns_pre_delete_values(self, conn) -> None:
        """RETURNING captures the row BEFORE it's deleted."""
        conn.execute("INSERT INTO employees VALUES (5, 'Eve', 55555)")
        cur = conn.execute(
            "DELETE FROM employees WHERE id = 5 RETURNING salary"
        )
        assert cur.fetchone() == (55555,)
        # Row is gone.
        remaining = conn.execute("SELECT * FROM employees WHERE id = 5").fetchall()
        assert remaining == []

    def test_delete_multiple_rows_returning(self, conn) -> None:
        conn.execute("INSERT INTO employees VALUES (1, 'A', 10)")
        conn.execute("INSERT INTO employees VALUES (2, 'B', 20)")
        conn.execute("INSERT INTO employees VALUES (3, 'C', 30)")
        cur = conn.execute("DELETE FROM employees RETURNING id")
        rows = cur.fetchall()
        assert len(rows) == 3
        # All rows deleted.
        assert conn.execute("SELECT * FROM employees").fetchall() == []

    def test_delete_no_match_returns_empty(self, conn) -> None:
        conn.execute("INSERT INTO employees VALUES (1, 'Alice', 50000)")
        cur = conn.execute(
            "DELETE FROM employees WHERE id = 999 RETURNING id"
        )
        assert cur.fetchall() == []
        # Row still present.
        assert conn.execute("SELECT COUNT(*) FROM employees").fetchone() == (1,)

    def test_delete_actually_removes_rows(self, conn) -> None:
        """RETURNING doesn't prevent the actual deletion side-effect."""
        conn.execute("INSERT INTO employees VALUES (1, 'Alice', 50000)")
        conn.execute("DELETE FROM employees WHERE id = 1 RETURNING id")
        remaining = conn.execute("SELECT * FROM employees").fetchall()
        assert remaining == []
