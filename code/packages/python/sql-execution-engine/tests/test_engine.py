"""Comprehensive tests for the SQL execution engine.

Each test class covers a specific feature area.  An InMemorySource with
four employees and two departments is used throughout.

Data Model
----------

employees:
    id | name  | dept_id | salary | active
    ---+-------+---------+--------+-------
    1  | Alice | 1       | 90000  | True
    2  | Bob   | 2       | 75000  | True
    3  | Carol | 1       | 95000  | False
    4  | Dave  | NULL    | 60000  | True

departments:
    id | name        | budget
    ---+-------------+-------
    1  | Engineering | 500000
    2  | Marketing   | 200000
"""

from __future__ import annotations

import pytest

from sql_execution_engine import (
    DataSource,
    QueryResult,
    execute,
    execute_all,
)
from sql_execution_engine.errors import (
    ColumnNotFoundError,
    ExecutionError,
    TableNotFoundError,
)


# ---------------------------------------------------------------------------
# Test data source
# ---------------------------------------------------------------------------


class InMemorySource(DataSource):
    """In-memory data source for testing.

    Holds two tables: ``employees`` and ``departments``.
    """

    EMPLOYEES: list[dict] = [
        {"id": 1, "name": "Alice", "dept_id": 1,    "salary": 90000, "active": True},
        {"id": 2, "name": "Bob",   "dept_id": 2,    "salary": 75000, "active": True},
        {"id": 3, "name": "Carol", "dept_id": 1,    "salary": 95000, "active": False},
        {"id": 4, "name": "Dave",  "dept_id": None, "salary": 60000, "active": True},
    ]
    DEPARTMENTS: list[dict] = [
        {"id": 1, "name": "Engineering", "budget": 500000},
        {"id": 2, "name": "Marketing",   "budget": 200000},
    ]

    def schema(self, table_name: str) -> list[str]:
        if table_name == "employees":
            return ["id", "name", "dept_id", "salary", "active"]
        if table_name == "departments":
            return ["id", "name", "budget"]
        raise TableNotFoundError(table_name)

    def scan(self, table_name: str) -> list[dict]:
        if table_name == "employees":
            return list(self.EMPLOYEES)
        if table_name == "departments":
            return list(self.DEPARTMENTS)
        raise TableNotFoundError(table_name)


SOURCE = InMemorySource()


# ---------------------------------------------------------------------------
# Helper
# ---------------------------------------------------------------------------


def run(sql: str) -> QueryResult:
    return execute(sql, SOURCE)


# ---------------------------------------------------------------------------
# Test 1: SELECT *
# ---------------------------------------------------------------------------


class TestSelectStar:
    """SELECT * returns all columns and all rows."""

    def test_columns(self) -> None:
        result = run("SELECT * FROM employees")
        assert "id" in result.columns
        assert "name" in result.columns
        assert "salary" in result.columns

    def test_row_count(self) -> None:
        result = run("SELECT * FROM employees")
        assert len(result.rows) == 4

    def test_all_values_present(self) -> None:
        result = run("SELECT * FROM employees")
        names = {row["name"] for row in result.rows}
        assert names == {"Alice", "Bob", "Carol", "Dave"}


# ---------------------------------------------------------------------------
# Test 2: SELECT specific columns
# ---------------------------------------------------------------------------


class TestSelectColumns:
    """SELECT id, name returns only those two columns."""

    def test_only_requested_columns(self) -> None:
        result = run("SELECT id, name FROM employees")
        assert result.columns == ["id", "name"]

    def test_values_correct(self) -> None:
        result = run("SELECT id, name FROM employees")
        ids = {row["id"] for row in result.rows}
        assert ids == {1, 2, 3, 4}


# ---------------------------------------------------------------------------
# Test 3: AS alias
# ---------------------------------------------------------------------------


class TestAlias:
    """AS alias renames the output column."""

    def test_alias_column_name(self) -> None:
        result = run("SELECT id, name AS employee_name FROM employees")
        assert "employee_name" in result.columns
        assert "name" not in result.columns

    def test_alias_values(self) -> None:
        result = run("SELECT id, name AS employee_name FROM employees")
        names = {row["employee_name"] for row in result.rows}
        assert "Alice" in names


# ---------------------------------------------------------------------------
# Test 4: WHERE salary > N
# ---------------------------------------------------------------------------


class TestWhereNumeric:
    """WHERE with numeric comparison filters rows."""

    def test_salary_greater_than(self) -> None:
        result = run("SELECT id, name FROM employees WHERE salary > 80000")
        names = {row["name"] for row in result.rows}
        assert names == {"Alice", "Carol"}

    def test_salary_less_than(self) -> None:
        result = run("SELECT id FROM employees WHERE salary < 80000")
        ids = {row["id"] for row in result.rows}
        assert ids == {2, 4}


# ---------------------------------------------------------------------------
# Test 5: WHERE active = true
# ---------------------------------------------------------------------------


class TestWhereBool:
    """WHERE against boolean column."""

    def test_active_true(self) -> None:
        result = run("SELECT name FROM employees WHERE active = TRUE")
        names = {row["name"] for row in result.rows}
        assert names == {"Alice", "Bob", "Dave"}

    def test_active_false(self) -> None:
        result = run("SELECT name FROM employees WHERE active = FALSE")
        names = {row["name"] for row in result.rows}
        assert names == {"Carol"}


# ---------------------------------------------------------------------------
# Test 6: WHERE IS NULL
# ---------------------------------------------------------------------------


class TestWhereIsNull:
    """WHERE col IS NULL filters to NULL rows."""

    def test_is_null(self) -> None:
        result = run("SELECT name FROM employees WHERE dept_id IS NULL")
        assert len(result.rows) == 1
        assert result.rows[0]["name"] == "Dave"


# ---------------------------------------------------------------------------
# Test 7: WHERE IS NOT NULL
# ---------------------------------------------------------------------------


class TestWhereIsNotNull:
    """WHERE col IS NOT NULL excludes NULL rows."""

    def test_is_not_null(self) -> None:
        result = run("SELECT name FROM employees WHERE dept_id IS NOT NULL")
        names = {row["name"] for row in result.rows}
        assert "Dave" not in names
        assert len(result.rows) == 3


# ---------------------------------------------------------------------------
# Test 8: WHERE BETWEEN
# ---------------------------------------------------------------------------


class TestWhereBetween:
    """WHERE col BETWEEN low AND high."""

    def test_between_inclusive(self) -> None:
        result = run("SELECT name FROM employees WHERE salary BETWEEN 70000 AND 90000")
        names = {row["name"] for row in result.rows}
        assert names == {"Alice", "Bob"}  # 90000 inclusive, 75000, not Carol(95k) or Dave(60k)


# ---------------------------------------------------------------------------
# Test 9: WHERE IN
# ---------------------------------------------------------------------------


class TestWhereIn:
    """WHERE col IN (...)."""

    def test_in_list(self) -> None:
        result = run("SELECT name FROM employees WHERE id IN (1, 3)")
        names = {row["name"] for row in result.rows}
        assert names == {"Alice", "Carol"}


# ---------------------------------------------------------------------------
# Test 10: WHERE LIKE
# ---------------------------------------------------------------------------


class TestWhereLike:
    """WHERE col LIKE 'pattern%'."""

    def test_like_prefix(self) -> None:
        result = run("SELECT name FROM employees WHERE name LIKE 'A%'")
        names = {row["name"] for row in result.rows}
        assert names == {"Alice"}

    def test_like_suffix(self) -> None:
        result = run("SELECT name FROM employees WHERE name LIKE '%ob'")
        names = {row["name"] for row in result.rows}
        assert names == {"Bob"}


# ---------------------------------------------------------------------------
# Test 11: WHERE AND / OR / NOT
# ---------------------------------------------------------------------------


class TestWhereLogical:
    """Logical operators in WHERE clause."""

    def test_and(self) -> None:
        result = run(
            "SELECT name FROM employees WHERE salary > 70000 AND active = TRUE"
        )
        names = {row["name"] for row in result.rows}
        assert names == {"Alice", "Bob"}

    def test_or(self) -> None:
        result = run(
            "SELECT name FROM employees WHERE salary > 90000 OR active = FALSE"
        )
        names = {row["name"] for row in result.rows}
        assert names == {"Carol"}  # Carol: 95000 > 90000 AND active=False, both match

    def test_not(self) -> None:
        result = run("SELECT name FROM employees WHERE NOT active = TRUE")
        names = {row["name"] for row in result.rows}
        assert names == {"Carol"}


# ---------------------------------------------------------------------------
# Test 12: ORDER BY
# ---------------------------------------------------------------------------


class TestOrderBy:
    """ORDER BY sorts rows ascending and descending."""

    def test_order_by_salary_desc(self) -> None:
        result = run("SELECT name FROM employees ORDER BY salary DESC")
        names = [row["name"] for row in result.rows]
        assert names[0] == "Carol"   # 95000 highest
        assert names[-1] == "Dave"  # 60000 lowest

    def test_order_by_name_asc(self) -> None:
        result = run("SELECT name FROM employees ORDER BY name ASC")
        names = [row["name"] for row in result.rows]
        assert names == sorted(names)


# ---------------------------------------------------------------------------
# Test 13: LIMIT and OFFSET
# ---------------------------------------------------------------------------


class TestLimitOffset:
    """LIMIT and OFFSET paginate results."""

    def test_limit(self) -> None:
        result = run("SELECT id FROM employees LIMIT 2")
        assert len(result.rows) == 2

    def test_limit_offset(self) -> None:
        result_all = run("SELECT id FROM employees ORDER BY id ASC")
        result_page = run("SELECT id FROM employees ORDER BY id ASC LIMIT 2 OFFSET 1")
        assert len(result_page.rows) == 2
        assert result_page.rows[0]["id"] == result_all.rows[1]["id"]


# ---------------------------------------------------------------------------
# Test 14: SELECT DISTINCT
# ---------------------------------------------------------------------------


class TestDistinct:
    """SELECT DISTINCT removes duplicate rows."""

    def test_distinct_dept_id(self) -> None:
        result = run("SELECT DISTINCT dept_id FROM employees")
        # Should have 3 distinct values: 1, 2, None
        assert len(result.rows) == 3


# ---------------------------------------------------------------------------
# Test 15: INNER JOIN
# ---------------------------------------------------------------------------


class TestInnerJoin:
    """INNER JOIN returns only matching rows."""

    def test_inner_join_basic(self) -> None:
        result = run(
            "SELECT employees.name, departments.name "
            "FROM employees INNER JOIN departments "
            "ON employees.dept_id = departments.id"
        )
        # Dave has NULL dept_id → no match → excluded
        assert len(result.rows) == 3

    def test_inner_join_columns(self) -> None:
        result = run(
            "SELECT employees.name AS emp, departments.name AS dept "
            "FROM employees INNER JOIN departments "
            "ON employees.dept_id = departments.id"
        )
        emp_names = {row["emp"] for row in result.rows}
        assert emp_names == {"Alice", "Bob", "Carol"}


# ---------------------------------------------------------------------------
# Test 16: LEFT JOIN
# ---------------------------------------------------------------------------


class TestLeftJoin:
    """LEFT JOIN includes all left rows, NULLs for unmatched right columns."""

    def test_left_join_includes_dave(self) -> None:
        result = run(
            "SELECT employees.name "
            "FROM employees LEFT JOIN departments "
            "ON employees.dept_id = departments.id"
        )
        names = {row["employees.name"] for row in result.rows}
        assert "Dave" in names
        assert len(result.rows) == 4

    def test_left_join_null_dept_for_dave(self) -> None:
        result = run(
            "SELECT employees.name, departments.name AS dept_name "
            "FROM employees LEFT JOIN departments "
            "ON employees.dept_id = departments.id"
        )
        dave_row = next(r for r in result.rows if r.get("employees.name") == "Dave")
        assert dave_row["dept_name"] is None


# ---------------------------------------------------------------------------
# Test 17: Aggregate functions — COUNT(*), AVG(salary)
# ---------------------------------------------------------------------------


class TestAggregates:
    """Aggregate functions on whole table (no GROUP BY)."""

    def test_count_star(self) -> None:
        result = run("SELECT COUNT(*) FROM employees")
        assert len(result.rows) == 1
        count_val = list(result.rows[0].values())[0]
        assert count_val == 4

    def test_avg_salary(self) -> None:
        result = run("SELECT AVG(salary) FROM employees")
        assert len(result.rows) == 1
        avg_val = list(result.rows[0].values())[0]
        expected = (90000 + 75000 + 95000 + 60000) / 4
        assert avg_val == pytest.approx(expected)


# ---------------------------------------------------------------------------
# Test 18: GROUP BY with COUNT(*) and SUM(salary)
# ---------------------------------------------------------------------------


class TestGroupBy:
    """GROUP BY partitions rows and aggregates per group."""

    def test_group_by_dept_count(self) -> None:
        result = run(
            "SELECT dept_id, COUNT(*) "
            "FROM employees "
            "GROUP BY dept_id"
        )
        # Three groups: dept_id=1, dept_id=2, dept_id=NULL
        assert len(result.rows) == 3

    def test_group_by_dept_sum(self) -> None:
        result = run(
            "SELECT dept_id, SUM(salary) "
            "FROM employees "
            "WHERE dept_id IS NOT NULL "
            "GROUP BY dept_id"
        )
        rows_by_dept = {row["dept_id"]: row for row in result.rows}
        # dept 1: Alice(90000) + Carol(95000) = 185000
        assert rows_by_dept[1]["SUM(salary)"] == 185000
        # dept 2: Bob(75000)
        assert rows_by_dept[2]["SUM(salary)"] == 75000


# ---------------------------------------------------------------------------
# Test 19: HAVING
# ---------------------------------------------------------------------------


class TestHaving:
    """HAVING filters groups after aggregation."""

    def test_having_sum_greater_than(self) -> None:
        result = run(
            "SELECT dept_id, SUM(salary) "
            "FROM employees "
            "WHERE dept_id IS NOT NULL "
            "GROUP BY dept_id "
            "HAVING SUM(salary) > 100000"
        )
        # Only dept 1 has sum > 100000 (185000)
        assert len(result.rows) == 1
        assert result.rows[0]["dept_id"] == 1


# ---------------------------------------------------------------------------
# Test 20: Arithmetic in SELECT
# ---------------------------------------------------------------------------


class TestArithmetic:
    """Arithmetic expressions in SELECT clause."""

    def test_salary_times_constant(self) -> None:
        result = run("SELECT salary * 1.1 AS adjusted FROM employees WHERE id = 1")
        assert len(result.rows) == 1
        assert result.rows[0]["adjusted"] == pytest.approx(99000.0)

    def test_arithmetic_addition(self) -> None:
        result = run("SELECT salary + 5000 AS bumped FROM employees WHERE id = 4")
        assert result.rows[0]["bumped"] == 65000


# ---------------------------------------------------------------------------
# Test 21: TableNotFoundError
# ---------------------------------------------------------------------------


class TestTableNotFoundError:
    """Querying an unknown table raises TableNotFoundError."""

    def test_unknown_table(self) -> None:
        with pytest.raises(TableNotFoundError) as exc_info:
            run("SELECT * FROM nonexistent_table")
        assert "nonexistent_table" in str(exc_info.value)

    def test_is_execution_error(self) -> None:
        with pytest.raises(ExecutionError):
            run("SELECT * FROM nonexistent_table")


# ---------------------------------------------------------------------------
# Test 22: ColumnNotFoundError
# ---------------------------------------------------------------------------


class TestColumnNotFoundError:
    """Referencing an unknown column raises ColumnNotFoundError."""

    def test_unknown_column_in_where(self) -> None:
        with pytest.raises(ColumnNotFoundError):
            run("SELECT id FROM employees WHERE nonexistent_col = 1")

    def test_is_execution_error(self) -> None:
        with pytest.raises(ExecutionError):
            run("SELECT id FROM employees WHERE nonexistent_col = 1")


# ---------------------------------------------------------------------------
# Test 23: execute_all
# ---------------------------------------------------------------------------


class TestExecuteAll:
    """execute_all runs multiple statements."""

    def test_two_selects(self) -> None:
        results = execute_all(
            "SELECT id FROM employees; SELECT id FROM departments",
            SOURCE,
        )
        assert len(results) == 2
        assert len(results[0].rows) == 4
        assert len(results[1].rows) == 2

    def test_returns_list(self) -> None:
        results = execute_all("SELECT id FROM employees", SOURCE)
        assert isinstance(results, list)
        assert len(results) == 1


# ---------------------------------------------------------------------------
# Test 24: QueryResult repr
# ---------------------------------------------------------------------------


class TestQueryResult:
    """QueryResult dataclass behaves correctly."""

    def test_repr_singular(self) -> None:
        result = run("SELECT id FROM employees WHERE id = 1")
        assert "1 row" in repr(result)

    def test_repr_plural(self) -> None:
        result = run("SELECT id FROM employees")
        assert "4 rows" in repr(result)

    def test_columns_list(self) -> None:
        result = run("SELECT id, name FROM employees")
        assert isinstance(result.columns, list)


# ---------------------------------------------------------------------------
# Test 25: Error classes
# ---------------------------------------------------------------------------


class TestErrorClasses:
    """Error class hierarchy and attributes."""

    def test_table_not_found_attributes(self) -> None:
        err = TableNotFoundError("mytable")
        assert err.table_name == "mytable"
        assert "mytable" in str(err)

    def test_column_not_found_attributes(self) -> None:
        err = ColumnNotFoundError("mycol")
        assert err.column_name == "mycol"
        assert "mycol" in str(err)

    def test_execution_error_base(self) -> None:
        err = ExecutionError("oops")
        assert isinstance(err, Exception)


# ---------------------------------------------------------------------------
# Test 26: MIN / MAX aggregates
# ---------------------------------------------------------------------------


class TestMinMax:
    """MIN and MAX aggregate functions."""

    def test_min_salary(self) -> None:
        result = run("SELECT MIN(salary) FROM employees")
        val = list(result.rows[0].values())[0]
        assert val == 60000

    def test_max_salary(self) -> None:
        result = run("SELECT MAX(salary) FROM employees")
        val = list(result.rows[0].values())[0]
        assert val == 95000


# ---------------------------------------------------------------------------
# Test 27: COUNT with column
# ---------------------------------------------------------------------------


class TestCountColumn:
    """COUNT(col) counts non-NULL values."""

    def test_count_dept_id_excludes_null(self) -> None:
        result = run("SELECT COUNT(dept_id) FROM employees")
        val = list(result.rows[0].values())[0]
        assert val == 3  # Dave has NULL dept_id → excluded


# ---------------------------------------------------------------------------
# Test 28: Additional coverage — OR, NOT LIKE, NOT IN, NOT BETWEEN
# ---------------------------------------------------------------------------


class TestAdditionalCoverage:
    """Additional tests to reach coverage requirements."""

    def test_or_condition(self) -> None:
        result = run("SELECT name FROM employees WHERE id = 1 OR id = 3")
        names = {row["name"] for row in result.rows}
        assert names == {"Alice", "Carol"}

    def test_not_like(self) -> None:
        result = run("SELECT name FROM employees WHERE name NOT LIKE 'A%'")
        names = {row["name"] for row in result.rows}
        assert "Alice" not in names
        assert len(result.rows) == 3

    def test_not_in(self) -> None:
        result = run("SELECT name FROM employees WHERE id NOT IN (1, 2)")
        names = {row["name"] for row in result.rows}
        assert names == {"Carol", "Dave"}

    def test_not_between(self) -> None:
        result = run("SELECT name FROM employees WHERE salary NOT BETWEEN 70000 AND 90000")
        names = {row["name"] for row in result.rows}
        assert names == {"Carol", "Dave"}

    def test_execute_all_empty(self) -> None:
        from sql_execution_engine import execute_all
        results = execute_all("SELECT id FROM employees; SELECT id FROM departments", SOURCE)
        assert len(results) == 2

    def test_sum_aggregate(self) -> None:
        result = run("SELECT SUM(salary) FROM employees")
        val = list(result.rows[0].values())[0]
        assert val == 320000

    def test_order_by_asc(self) -> None:
        result = run("SELECT name FROM employees ORDER BY name ASC")
        names = [row["name"] for row in result.rows]
        assert names == sorted(names, key=str.lower)

    def test_like_infix(self) -> None:
        result = run("SELECT name FROM employees WHERE name LIKE '%li%'")
        names = {row["name"] for row in result.rows}
        assert "Alice" in names

    def test_inner_join_column_count(self) -> None:
        result = run(
            "SELECT employees.name, departments.name "
            "FROM employees INNER JOIN departments "
            "ON employees.dept_id = departments.id"
        )
        assert len(result.rows) == 3
