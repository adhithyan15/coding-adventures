"""
Tests for sql_csv_source.

These tests run end-to-end against real CSV fixture files, exercising the
full pipeline: CSV file → CsvDataSource → sql-execution-engine → QueryResult.

Fixture data:

    employees.csv
        id  | name  | dept_id | salary | active
        1   | Alice | 1       | 90000  | true
        2   | Bob   | 2       | 75000  | true
        3   | Carol | 1       | 95000  | false
        4   | Dave  | (null)  | 60000  | true

    departments.csv
        id | name        | budget
        1  | Engineering | 500000
        2  | Marketing   | 200000
"""

import os
from pathlib import Path

import pytest

from sql_csv_source import CsvDataSource, execute_csv
from sql_csv_source.csv_data_source import _coerce
from sql_execution_engine.errors import TableNotFoundError

# ---------------------------------------------------------------------------
# Fixture directory (relative to this test file)
# ---------------------------------------------------------------------------

FIXTURES = Path(__file__).parent / "fixtures"


# ---------------------------------------------------------------------------
# Unit tests for the _coerce helper
# ---------------------------------------------------------------------------


class TestCoerce:
    """Test type coercion of individual CSV string values."""

    def test_empty_string_becomes_none(self) -> None:
        """Empty CSV field → SQL NULL → Python None."""
        assert _coerce("") is None

    def test_true_string(self) -> None:
        assert _coerce("true") is True

    def test_false_string(self) -> None:
        assert _coerce("false") is False

    def test_true_uppercase(self) -> None:
        """Boolean coercion is case-insensitive."""
        assert _coerce("True") is True

    def test_false_uppercase(self) -> None:
        assert _coerce("False") is False

    def test_integer(self) -> None:
        assert _coerce("42") == 42
        assert isinstance(_coerce("42"), int)

    def test_negative_integer(self) -> None:
        assert _coerce("-7") == -7

    def test_zero(self) -> None:
        assert _coerce("0") == 0

    def test_float(self) -> None:
        result = _coerce("3.14")
        assert abs(result - 3.14) < 1e-9
        assert isinstance(result, float)

    def test_string_passthrough(self) -> None:
        assert _coerce("hello") == "hello"
        assert isinstance(_coerce("hello"), str)

    def test_string_with_spaces(self) -> None:
        assert _coerce("Alice Smith") == "Alice Smith"


# ---------------------------------------------------------------------------
# Unit tests for CsvDataSource.schema()
# ---------------------------------------------------------------------------


class TestSchema:
    """Test column-name discovery from CSV header rows."""

    def test_employees_schema(self) -> None:
        source = CsvDataSource(FIXTURES)
        cols = source.schema("employees")
        assert cols == ["id", "name", "dept_id", "salary", "active"]

    def test_departments_schema(self) -> None:
        source = CsvDataSource(FIXTURES)
        cols = source.schema("departments")
        assert cols == ["id", "name", "budget"]

    def test_unknown_table_raises(self) -> None:
        source = CsvDataSource(FIXTURES)
        with pytest.raises(TableNotFoundError) as exc_info:
            source.schema("nonexistent")
        assert "nonexistent" in str(exc_info.value)


# ---------------------------------------------------------------------------
# Unit tests for CsvDataSource.scan() — type coercion on rows
# ---------------------------------------------------------------------------


class TestScan:
    """Test that scan() returns rows with correctly coerced values."""

    def test_scan_employees_count(self) -> None:
        source = CsvDataSource(FIXTURES)
        rows = source.scan("employees")
        assert len(rows) == 4

    def test_alice_row_types(self) -> None:
        source = CsvDataSource(FIXTURES)
        rows = source.scan("employees")
        alice = rows[0]
        assert alice["id"] == 1
        assert isinstance(alice["id"], int)
        assert alice["name"] == "Alice"
        assert isinstance(alice["name"], str)
        assert alice["dept_id"] == 1
        assert alice["salary"] == 90000
        assert alice["active"] is True

    def test_carol_active_is_false(self) -> None:
        source = CsvDataSource(FIXTURES)
        rows = source.scan("employees")
        carol = rows[2]
        assert carol["active"] is False

    def test_dave_dept_id_is_none(self) -> None:
        """Dave has an empty dept_id field — should coerce to None (SQL NULL)."""
        source = CsvDataSource(FIXTURES)
        rows = source.scan("employees")
        dave = rows[3]
        assert dave["dept_id"] is None

    def test_departments_budget_is_int(self) -> None:
        source = CsvDataSource(FIXTURES)
        rows = source.scan("departments")
        assert rows[0]["budget"] == 500000
        assert isinstance(rows[0]["budget"], int)

    def test_unknown_table_raises(self) -> None:
        source = CsvDataSource(FIXTURES)
        with pytest.raises(TableNotFoundError):
            source.scan("missing")


# ---------------------------------------------------------------------------
# End-to-end SQL query tests via execute_csv()
# ---------------------------------------------------------------------------


class TestExecuteCsv:
    """Full pipeline: SQL string → CSV files → QueryResult."""

    def test_select_star_employees(self) -> None:
        """SELECT * FROM employees returns 4 rows with all columns."""
        result = execute_csv("SELECT * FROM employees", FIXTURES)
        assert result.columns == ["id", "name", "dept_id", "salary", "active"]
        assert len(result.rows) == 4

    def test_select_star_employees_types(self) -> None:
        """Values in the result are coerced, not raw strings."""
        result = execute_csv("SELECT * FROM employees", FIXTURES)
        alice = result.rows[0]
        assert alice["id"] == 1
        assert alice["name"] == "Alice"
        assert alice["active"] is True
        # Dave's dept_id is NULL
        dave = result.rows[3]
        assert dave["dept_id"] is None

    def test_select_active_employees(self) -> None:
        """SELECT name WHERE active = true — Alice, Bob, Dave."""
        result = execute_csv(
            "SELECT name FROM employees WHERE active = true", FIXTURES
        )
        names = [r["name"] for r in result.rows]
        assert sorted(names) == ["Alice", "Bob", "Dave"]

    def test_select_where_dept_id_is_null(self) -> None:
        """SELECT WHERE dept_id IS NULL — only Dave."""
        result = execute_csv(
            "SELECT * FROM employees WHERE dept_id IS NULL", FIXTURES
        )
        assert len(result.rows) == 1
        assert result.rows[0]["name"] == "Dave"

    def test_inner_join(self) -> None:
        """INNER JOIN excludes Dave (no dept_id)."""
        result = execute_csv(
            "SELECT e.name, d.name "
            "FROM employees AS e "
            "INNER JOIN departments AS d ON e.dept_id = d.id",
            FIXTURES,
        )
        # Alice (eng), Bob (mkt), Carol (eng) — Dave excluded (NULL dept_id)
        assert len(result.rows) == 3
        emp_names = {r["e.name"] for r in result.rows}
        assert emp_names == {"Alice", "Bob", "Carol"}

    def test_group_by_dept_id(self) -> None:
        """GROUP BY dept_id includes the NULL group for Dave."""
        result = execute_csv(
            "SELECT dept_id, COUNT(*) AS cnt FROM employees GROUP BY dept_id",
            FIXTURES,
        )
        # Three groups: dept_id=1 (Alice+Carol), dept_id=2 (Bob), NULL (Dave)
        assert len(result.rows) == 3
        # Find the count for dept 1
        dept1 = next(r for r in result.rows if r["dept_id"] == 1)
        assert dept1["cnt"] == 2

    def test_order_by_salary_desc_limit_2(self) -> None:
        """ORDER BY salary DESC LIMIT 2 — Carol (95000), Alice (90000)."""
        result = execute_csv(
            "SELECT name, salary FROM employees ORDER BY salary DESC LIMIT 2",
            FIXTURES,
        )
        assert len(result.rows) == 2
        assert result.rows[0]["name"] == "Carol"
        assert result.rows[0]["salary"] == 95000
        assert result.rows[1]["name"] == "Alice"
        assert result.rows[1]["salary"] == 90000

    def test_unknown_table_raises(self) -> None:
        """Querying a non-existent table raises TableNotFoundError."""
        with pytest.raises(TableNotFoundError):
            execute_csv("SELECT * FROM ghosts", FIXTURES)

    def test_execute_csv_convenience_returns_same_as_direct(self) -> None:
        """execute_csv is a thin wrapper — results must match direct usage."""
        direct_source = CsvDataSource(FIXTURES)
        from sql_execution_engine import execute as ee_execute

        direct_result = ee_execute("SELECT * FROM departments", direct_source)
        wrapper_result = execute_csv("SELECT * FROM departments", FIXTURES)

        assert wrapper_result.columns == direct_result.columns
        assert wrapper_result.rows == direct_result.rows
