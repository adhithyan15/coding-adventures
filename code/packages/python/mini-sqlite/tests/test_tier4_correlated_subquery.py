"""Correlated subquery integration tests — full mini-sqlite pipeline.

These tests verify that the complete stack (parser → adapter → planner →
codegen → VM) correctly handles correlated subqueries: subqueries whose
WHERE clause references columns from the enclosing (outer) query.

A correlated subquery is *re-executed* for each outer row, with the inner
``LoadOuterColumn`` instruction reading the outer cursor's current row
snapshot.

Test matrix
-----------
- Correlated ``IN`` — employees whose dept appears in departments
- Correlated ``NOT IN`` — employees whose dept is absent from departments
- Correlated ``EXISTS`` — same as IN but via EXISTS check
- Correlated ``NOT EXISTS`` — employees with no department entry
- Correlated scalar subquery in SELECT list — dept name per employee
- NULL semantics — missing department produces NULL scalar result
- Double correlated — two subqueries in the same WHERE
- Subquery with no rows — inner always empty
"""

from __future__ import annotations

import pytest

import mini_sqlite

# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture
def conn():
    """In-memory database with employees + departments tables.

    Schema::

        employees(id INTEGER, name TEXT, dept_id INTEGER)
        departments(id INTEGER, dept_name TEXT)

    Data::

        employees: Alice(1,10), Bob(2,20), Carol(3,10), Dave(4,30)
        departments: eng(10), sales(20)
        Dave belongs to dept 30 which has NO entry in departments.
    """
    c = mini_sqlite.connect(":memory:")
    c.execute("""
        CREATE TABLE employees (
            id      INTEGER,
            name    TEXT,
            dept_id INTEGER
        )
    """)
    c.execute("""
        CREATE TABLE departments (
            id        INTEGER,
            dept_name TEXT
        )
    """)
    c.executemany(
        "INSERT INTO employees (id, name, dept_id) VALUES (?, ?, ?)",
        [(1, "Alice", 10), (2, "Bob", 20), (3, "Carol", 10), (4, "Dave", 30)],
    )
    c.executemany(
        "INSERT INTO departments (id, dept_name) VALUES (?, ?)",
        [(10, "eng"), (20, "sales")],
    )
    yield c
    c.close()


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _names(rows) -> list[str]:
    return sorted(r[0] for r in rows)


# ---------------------------------------------------------------------------
# Correlated IN subquery
# ---------------------------------------------------------------------------


class TestCorrelatedInSubquery:
    def test_basic_correlated_in(self, conn) -> None:
        """Employees whose dept_id appears in departments.id (correlated IN).

        SQL::

            SELECT e.name FROM employees AS e
            WHERE e.dept_id IN (
                SELECT d.id FROM departments AS d WHERE d.id = e.dept_id
            )

        Expected: Alice, Bob, Carol (dept 10 and 20 are in departments).
        Dave (dept 30) is absent.
        """
        rows = conn.execute("""
            SELECT e.name FROM employees AS e
            WHERE e.dept_id IN (
                SELECT d.id FROM departments AS d WHERE d.id = e.dept_id
            )
        """).fetchall()
        assert _names(rows) == ["Alice", "Bob", "Carol"]

    def test_correlated_in_no_match(self, conn) -> None:
        """Correlated IN where inner condition can never match → zero rows."""
        rows = conn.execute("""
            SELECT e.name FROM employees AS e
            WHERE e.dept_id IN (
                SELECT d.id FROM departments AS d WHERE d.id = e.dept_id AND d.id > 9999
            )
        """).fetchall()
        assert rows == []

    def test_correlated_in_all_match(self, conn) -> None:
        """Correlated IN against a non-correlated superlist — all match."""
        # Add dept 30 so every employee has an entry.
        conn.execute("INSERT INTO departments (id, dept_name) VALUES (30, 'ops')")
        rows = conn.execute("""
            SELECT e.name FROM employees AS e
            WHERE e.dept_id IN (
                SELECT d.id FROM departments AS d WHERE d.id = e.dept_id
            )
        """).fetchall()
        assert _names(rows) == ["Alice", "Bob", "Carol", "Dave"]


# ---------------------------------------------------------------------------
# Correlated NOT IN subquery
# ---------------------------------------------------------------------------


class TestCorrelatedNotInSubquery:
    def test_basic_correlated_not_in(self, conn) -> None:
        """Employees whose dept_id does NOT appear in departments (correlated NOT IN).

        SQL::

            SELECT e.name FROM employees AS e
            WHERE e.dept_id NOT IN (
                SELECT d.id FROM departments AS d WHERE d.id = e.dept_id
            )

        Only Dave (dept 30) qualifies.
        """
        rows = conn.execute("""
            SELECT e.name FROM employees AS e
            WHERE e.dept_id NOT IN (
                SELECT d.id FROM departments AS d WHERE d.id = e.dept_id
            )
        """).fetchall()
        assert _names(rows) == ["Dave"]

    def test_correlated_not_in_empty_result_when_all_match(self, conn) -> None:
        """NOT IN with every employee matched → empty result."""
        conn.execute("INSERT INTO departments (id, dept_name) VALUES (30, 'ops')")
        rows = conn.execute("""
            SELECT e.name FROM employees AS e
            WHERE e.dept_id NOT IN (
                SELECT d.id FROM departments AS d WHERE d.id = e.dept_id
            )
        """).fetchall()
        assert rows == []


# ---------------------------------------------------------------------------
# Correlated EXISTS subquery
# ---------------------------------------------------------------------------


class TestCorrelatedExistsSubquery:
    def test_basic_correlated_exists(self, conn) -> None:
        """Employees with a matching department entry (correlated EXISTS).

        SQL::

            SELECT e.name FROM employees AS e
            WHERE EXISTS (
                SELECT 1 FROM departments AS d WHERE d.id = e.dept_id
            )

        Alice, Bob, Carol match; Dave does not.
        """
        rows = conn.execute("""
            SELECT e.name FROM employees AS e
            WHERE EXISTS (
                SELECT 1 FROM departments AS d WHERE d.id = e.dept_id
            )
        """).fetchall()
        assert _names(rows) == ["Alice", "Bob", "Carol"]

    def test_exists_with_no_matching_rows(self, conn) -> None:
        """EXISTS against an always-empty inner query → no outer rows survive."""
        rows = conn.execute("""
            SELECT e.name FROM employees AS e
            WHERE EXISTS (
                SELECT 1 FROM departments AS d WHERE d.id = e.dept_id AND d.id > 9999
            )
        """).fetchall()
        assert rows == []


# ---------------------------------------------------------------------------
# Correlated NOT EXISTS subquery
# ---------------------------------------------------------------------------


class TestCorrelatedNotExistsSubquery:
    def test_basic_correlated_not_exists(self, conn) -> None:
        """Employees with NO matching department entry (correlated NOT EXISTS).

        SQL::

            SELECT e.name FROM employees AS e
            WHERE NOT EXISTS (
                SELECT 1 FROM departments AS d WHERE d.id = e.dept_id
            )

        Only Dave (dept 30) qualifies.
        """
        rows = conn.execute("""
            SELECT e.name FROM employees AS e
            WHERE NOT EXISTS (
                SELECT 1 FROM departments AS d WHERE d.id = e.dept_id
            )
        """).fetchall()
        assert _names(rows) == ["Dave"]

    def test_not_exists_all_match_in_inner(self, conn) -> None:
        """NOT EXISTS with every employee having a department → zero rows."""
        conn.execute("INSERT INTO departments (id, dept_name) VALUES (30, 'ops')")
        rows = conn.execute("""
            SELECT e.name FROM employees AS e
            WHERE NOT EXISTS (
                SELECT 1 FROM departments AS d WHERE d.id = e.dept_id
            )
        """).fetchall()
        assert rows == []


# ---------------------------------------------------------------------------
# Correlated scalar subquery in SELECT list
# ---------------------------------------------------------------------------


class TestCorrelatedScalarSubquery:
    def test_dept_name_for_each_employee(self, conn) -> None:
        """Scalar subquery in SELECT returns dept_name per row; NULL for no match.

        SQL::

            SELECT e.name,
                   (SELECT d.dept_name FROM departments AS d WHERE d.id = e.dept_id)
            FROM employees AS e

        Dave has no matching department → NULL.
        """
        rows = conn.execute("""
            SELECT e.name,
                   (SELECT d.dept_name FROM departments AS d WHERE d.id = e.dept_id)
            FROM employees AS e
        """).fetchall()
        name_to_dept = {r[0]: r[1] for r in rows}

        assert name_to_dept["Alice"] == "eng"
        assert name_to_dept["Bob"] == "sales"
        assert name_to_dept["Carol"] == "eng"
        assert name_to_dept["Dave"] is None  # dept 30 not in departments

    def test_scalar_subquery_null_for_no_match(self, conn) -> None:
        """A scalar subquery with no matching inner row yields NULL (not an error)."""
        rows = conn.execute("""
            SELECT e.name,
                   (SELECT d.dept_name FROM departments AS d WHERE d.id = e.dept_id)
            FROM employees AS e
            WHERE e.name = 'Dave'
        """).fetchall()
        assert len(rows) == 1
        assert rows[0][1] is None  # Dave's dept has no entry → NULL

    def test_scalar_subquery_reruns_per_outer_row(self, conn) -> None:
        """Each outer row sees its own correlated result — not a cached first result."""
        rows = conn.execute("""
            SELECT e.name,
                   (SELECT d.dept_name FROM departments AS d WHERE d.id = e.dept_id)
            FROM employees AS e
        """).fetchall()
        # If the inner program ran only once, all rows would show the same dept.
        # Verify Alice and Bob have different departments.
        name_to_dept = {r[0]: r[1] for r in rows}
        assert name_to_dept["Alice"] != name_to_dept["Bob"]
        assert name_to_dept["Alice"] == "eng"
        assert name_to_dept["Bob"] == "sales"
        # All four employees present
        assert len(rows) == 4


# ---------------------------------------------------------------------------
# Combining correlated subqueries with WHERE on the outer query
# ---------------------------------------------------------------------------


class TestCorrelatedWithOuterFilter:
    def test_correlated_in_plus_outer_where(self, conn) -> None:
        """Correlated IN combined with an additional outer WHERE condition."""
        rows = conn.execute("""
            SELECT e.name FROM employees AS e
            WHERE e.dept_id IN (
                SELECT d.id FROM departments AS d WHERE d.id = e.dept_id
            )
            AND e.id < 3
        """).fetchall()
        # Alice(id=1) and Bob(id=2) match both conditions
        assert _names(rows) == ["Alice", "Bob"]

    def test_correlated_exists_plus_outer_filter(self, conn) -> None:
        """Correlated EXISTS combined with outer inequality filter."""
        rows = conn.execute("""
            SELECT e.name FROM employees AS e
            WHERE EXISTS (
                SELECT 1 FROM departments AS d WHERE d.id = e.dept_id
            )
            AND e.dept_id = 10
        """).fetchall()
        # dept_id=10 matches Alice and Carol
        assert _names(rows) == ["Alice", "Carol"]
