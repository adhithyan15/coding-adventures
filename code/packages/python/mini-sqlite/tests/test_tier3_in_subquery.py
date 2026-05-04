"""IN (subquery) / NOT IN (subquery) integration tests.

These tests exercise the full mini-sqlite pipeline for the subquery form of
the IN operator:

    expr IN  (SELECT col FROM t [WHERE ...])
    expr NOT IN (SELECT col FROM t [WHERE ...])

The grammar already supported this form; previously the adapter raised
ProgrammingError("subquery in IN clause is not yet supported").  These tests
verify that the adapter, planner, codegen, and VM all handle it correctly.
"""

from __future__ import annotations

import pytest

import mini_sqlite


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture
def conn():
    """In-memory database with employees + departments + managers tables."""
    c = mini_sqlite.connect(":memory:")
    c.execute("""
        CREATE TABLE employees (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            dept_id INTEGER
        )
    """)
    c.execute("""
        CREATE TABLE departments (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL
        )
    """)
    c.execute("""
        CREATE TABLE managers (
            dept_id INTEGER
        )
    """)
    c.executemany(
        "INSERT INTO employees (id, name, dept_id) VALUES (?, ?, ?)",
        [
            (1, "Alice", 10),
            (2, "Bob",   20),
            (3, "Carol", 10),
            (4, "Dave",  30),
        ],
    )
    c.executemany(
        "INSERT INTO departments (id, name) VALUES (?, ?)",
        [(10, "eng"), (20, "sales"), (30, "hr")],
    )
    c.executemany(
        "INSERT INTO managers (dept_id) VALUES (?)",
        [(10,), (20,)],
    )
    return c


# ---------------------------------------------------------------------------
# Basic IN (subquery)
# ---------------------------------------------------------------------------


def test_in_subquery_basic(conn) -> None:
    """Employees whose dept_id is in the managed departments."""
    rows = conn.execute("""
        SELECT name
        FROM employees
        WHERE dept_id IN (SELECT dept_id FROM managers)
        ORDER BY name
    """).fetchall()

    assert [r[0] for r in rows] == ["Alice", "Bob", "Carol"]


def test_in_subquery_no_matches(conn) -> None:
    """IN against a subquery that returns no rows — all result in FALSE."""
    rows = conn.execute("""
        SELECT name
        FROM employees
        WHERE dept_id IN (SELECT id FROM departments WHERE name = 'nonexistent')
    """).fetchall()

    assert rows == []


def test_in_subquery_all_match(conn) -> None:
    """IN against a subquery that returns all dept IDs — every row matches."""
    rows = conn.execute("""
        SELECT name
        FROM employees
        WHERE dept_id IN (SELECT id FROM departments)
        ORDER BY name
    """).fetchall()

    assert [r[0] for r in rows] == ["Alice", "Bob", "Carol", "Dave"]


def test_in_subquery_literal_in_select(conn) -> None:
    """Subquery SELECTs a single known id; tests simple value membership."""
    rows = conn.execute("""
        SELECT name
        FROM employees
        WHERE id IN (SELECT id FROM employees WHERE id = 1)
    """).fetchall()

    assert rows == [("Alice",)]


def test_in_subquery_with_where_filter(conn) -> None:
    """Subquery with its own WHERE clause filters the set correctly."""
    rows = conn.execute("""
        SELECT name
        FROM employees
        WHERE dept_id IN (
            SELECT id FROM departments WHERE name = 'eng'
        )
        ORDER BY name
    """).fetchall()

    assert [r[0] for r in rows] == ["Alice", "Carol"]


# ---------------------------------------------------------------------------
# NOT IN (subquery)
# ---------------------------------------------------------------------------


def test_not_in_subquery_basic(conn) -> None:
    """NOT IN: employees in departments that are NOT managed."""
    rows = conn.execute("""
        SELECT name
        FROM employees
        WHERE dept_id NOT IN (SELECT dept_id FROM managers)
        ORDER BY name
    """).fetchall()

    assert rows == [("Dave",)]


def test_not_in_subquery_empty_set(conn) -> None:
    """NOT IN against empty subquery — all rows qualify (nothing excluded)."""
    rows = conn.execute("""
        SELECT name
        FROM employees
        WHERE dept_id NOT IN (SELECT id FROM departments WHERE name = 'nonexistent')
        ORDER BY name
    """).fetchall()

    assert [r[0] for r in rows] == ["Alice", "Bob", "Carol", "Dave"]


def test_not_in_subquery_all_excluded(conn) -> None:
    """NOT IN against a set containing every dept_id — zero rows qualify."""
    rows = conn.execute("""
        SELECT name
        FROM employees
        WHERE dept_id NOT IN (SELECT id FROM departments)
    """).fetchall()

    assert rows == []


# ---------------------------------------------------------------------------
# NULL semantics
# ---------------------------------------------------------------------------


def test_in_subquery_null_test_value(conn) -> None:
    """NULL IN (...) evaluates to NULL, which is falsy — row excluded."""
    conn.execute("INSERT INTO employees (id, name, dept_id) VALUES (99, 'Null Dept', NULL)")
    rows = conn.execute("""
        SELECT name
        FROM employees
        WHERE dept_id IN (SELECT dept_id FROM managers)
    """).fetchall()

    # Employee 99 has NULL dept_id; NULL IN (...) = NULL → excluded from results.
    names = {r[0] for r in rows}
    assert "Null Dept" not in names


def test_not_in_subquery_null_in_set(conn) -> None:
    """NOT IN when the subquery result contains NULL: result is NULL (excluded).

    SQL three-valued logic: x NOT IN (1, 2, NULL) = NULL, not TRUE.
    Rows with NULL in the subquery set behave as UNKNOWN.
    """
    # Insert a row with NULL dept_id into managers to add NULL to the IN set.
    conn.execute("INSERT INTO managers (dept_id) VALUES (NULL)")
    rows = conn.execute("""
        SELECT name
        FROM employees
        WHERE dept_id NOT IN (SELECT dept_id FROM managers)
    """).fetchall()

    # Because NULL is in the subquery set, NOT IN resolves to NULL for all
    # non-matching rows → no rows at all.  (Alice, Bob, Carol matched; Dave's
    # dept 30 was not in managers but NULL in the set makes it UNKNOWN.)
    assert rows == []


# ---------------------------------------------------------------------------
# IN subquery in more complex queries
# ---------------------------------------------------------------------------


def test_in_subquery_with_aggregate_subquery(conn) -> None:
    """Subquery with aggregate: ids where count is above average."""
    # dept_id=10 has 2 employees, dept_id=20 has 1, dept_id=30 has 1.
    rows = conn.execute("""
        SELECT name
        FROM departments
        WHERE id IN (
            SELECT dept_id
            FROM employees
            GROUP BY dept_id
            HAVING COUNT(*) > 1
        )
    """).fetchall()

    assert rows == [("eng",)]


def test_in_subquery_combined_with_other_predicates(conn) -> None:
    """IN subquery combined with AND predicate."""
    rows = conn.execute("""
        SELECT name
        FROM employees
        WHERE dept_id IN (SELECT dept_id FROM managers)
          AND id > 2
        ORDER BY name
    """).fetchall()

    assert rows == [("Carol",)]


def test_in_subquery_in_having(conn) -> None:
    """IN subquery used in HAVING clause after GROUP BY."""
    rows = conn.execute("""
        SELECT dept_id, COUNT(*) AS cnt
        FROM employees
        WHERE dept_id IN (SELECT dept_id FROM managers)
        GROUP BY dept_id
        ORDER BY dept_id
    """).fetchall()

    assert rows == [(10, 2), (20, 1)]
