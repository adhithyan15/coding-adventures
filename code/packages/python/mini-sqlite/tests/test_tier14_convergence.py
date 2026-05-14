"""Tier 14 convergence tests — oracle-verified against real sqlite3.

Each test case runs the same SQL against both mini-sqlite (in-memory) and
the real ``sqlite3`` standard-library module, then asserts the results are
equal.  Any divergence is a bug in our stack.

Tier 14 fixes the following categories of discrepancy found by find_gaps.py:

1. EmptyResult plan node — WHERE 1=0 / always-false predicates
2. HAVING clause that references a SELECT alias  (alias resolution)
3. IS DISTINCT FROM / IS NOT DISTINCT FROM  (NULL-safe comparison)
4. Scalar MAX/MIN with NULL arguments  (NULL propagation)
5. ABS() on non-numeric text  (coerce to 0.0, not pass-through)
6. HEX(NULL)  (return '' not NULL)
7. DATE +1 month overflow  (overflow into next month, not clamp)
8. LIKE NULL  (always NULL, not error)
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _both(sql: str, params: tuple = (), setup: list[str] | None = None) -> tuple[list, list]:
    """Run *sql* against both sqlite3 and mini-sqlite; return (sqlite3_rows, mini_rows)."""
    ref_con = sqlite3.connect(":memory:")
    got_con = mini_sqlite.connect(":memory:")
    for stmt in setup or []:
        ref_con.execute(stmt)
        got_con.execute(stmt)
    ref_rows = ref_con.execute(sql, params).fetchall()
    got_rows = got_con.execute(sql, params).fetchall()
    return ref_rows, got_rows


def check(sql: str, params: tuple = (), setup: list[str] | None = None) -> None:
    """Assert mini-sqlite matches sqlite3 for *sql* (with optional DDL/DML setup)."""
    want, got = _both(sql, params=params, setup=setup)
    assert got == want, f"SQL: {sql!r}\n  got:  {got}\n  want: {want}"


# ---------------------------------------------------------------------------
# 1. EmptyResult — WHERE with always-false predicate
# ---------------------------------------------------------------------------


class TestEmptyResult:
    def test_where_one_equals_zero(self) -> None:
        """SELECT from a real table with WHERE 1=0 → empty result."""
        setup = [
            "CREATE TABLE t (x INTEGER)",
            "INSERT INTO t VALUES (1)",
            "INSERT INTO t VALUES (2)",
        ]
        check("SELECT x FROM t WHERE 1=0", setup=setup)

    def test_where_literal_false(self) -> None:
        """WHERE FALSE → empty result (planner emits EmptyResult node)."""
        setup = [
            "CREATE TABLE t (x INTEGER)",
            "INSERT INTO t VALUES (42)",
        ]
        check("SELECT x FROM t WHERE 0", setup=setup)

    def test_count_star_where_false(self) -> None:
        """COUNT(*) with WHERE 1=0 → 0 (aggregate over empty input)."""
        setup = [
            "CREATE TABLE t (x INTEGER)",
            "INSERT INTO t VALUES (1)",
            "INSERT INTO t VALUES (2)",
        ]
        check("SELECT COUNT(*) FROM t WHERE 1=0", setup=setup)


# ---------------------------------------------------------------------------
# 2. HAVING clause referencing a SELECT alias
# ---------------------------------------------------------------------------


class TestHavingAlias:
    def test_having_with_alias(self) -> None:
        """HAVING can reference a SELECT-level alias."""
        setup = [
            "CREATE TABLE employees (dept TEXT, salary REAL)",
            "INSERT INTO employees VALUES ('eng', 90000)",
            "INSERT INTO employees VALUES ('eng', 80000)",
            "INSERT INTO employees VALUES ('hr', 50000)",
        ]
        sql = (
            "SELECT dept, AVG(salary) AS avg_sal "
            "FROM employees GROUP BY dept HAVING avg_sal > 60000"
        )
        check(sql, setup=setup)

    def test_having_alias_equals(self) -> None:
        """HAVING alias = literal."""
        setup = [
            "CREATE TABLE items (cat TEXT, qty INTEGER)",
            "INSERT INTO items VALUES ('A', 3)",
            "INSERT INTO items VALUES ('A', 2)",
            "INSERT INTO items VALUES ('B', 1)",
        ]
        sql = "SELECT cat, SUM(qty) AS total FROM items GROUP BY cat HAVING total = 5"
        check(sql, setup=setup)


# ---------------------------------------------------------------------------
# 3. IS DISTINCT FROM / IS NOT DISTINCT FROM
# ---------------------------------------------------------------------------


class TestIsDistinctFrom:
    def test_distinct_unequal(self) -> None:
        check("SELECT 1 IS DISTINCT FROM 2")

    def test_distinct_equal(self) -> None:
        check("SELECT 1 IS DISTINCT FROM 1")

    def test_distinct_null_left(self) -> None:
        check("SELECT NULL IS DISTINCT FROM 1")

    def test_distinct_null_right(self) -> None:
        check("SELECT 1 IS DISTINCT FROM NULL")

    def test_distinct_both_null(self) -> None:
        check("SELECT NULL IS DISTINCT FROM NULL")

    def test_not_distinct_equal(self) -> None:
        check("SELECT 1 IS NOT DISTINCT FROM 1")

    def test_not_distinct_both_null(self) -> None:
        check("SELECT NULL IS NOT DISTINCT FROM NULL")

    def test_not_distinct_mixed_null(self) -> None:
        check("SELECT NULL IS NOT DISTINCT FROM 1")

    def test_distinct_in_where(self) -> None:
        """IS DISTINCT FROM used as a WHERE predicate with column values."""
        setup = [
            "CREATE TABLE t (a INTEGER, b INTEGER)",
            "INSERT INTO t VALUES (1, 1)",
            "INSERT INTO t VALUES (1, 2)",
            "INSERT INTO t VALUES (NULL, 1)",
            "INSERT INTO t VALUES (NULL, NULL)",
        ]
        check("SELECT a, b FROM t WHERE a IS DISTINCT FROM b", setup=setup)

    def test_not_distinct_in_where(self) -> None:
        """IS NOT DISTINCT FROM used as a WHERE predicate."""
        setup = [
            "CREATE TABLE t (a INTEGER, b INTEGER)",
            "INSERT INTO t VALUES (1, 1)",
            "INSERT INTO t VALUES (1, 2)",
            "INSERT INTO t VALUES (NULL, NULL)",
        ]
        check("SELECT a, b FROM t WHERE a IS NOT DISTINCT FROM b", setup=setup)


# ---------------------------------------------------------------------------
# 4. Scalar MAX/MIN with NULL arguments
# ---------------------------------------------------------------------------


class TestScalarMaxMinNull:
    def test_max_with_null_right(self) -> None:
        check("SELECT MAX(1, NULL)")

    def test_max_with_null_left(self) -> None:
        check("SELECT MAX(NULL, 1)")

    def test_max_all_null(self) -> None:
        check("SELECT MAX(NULL, NULL)")

    def test_max_no_null(self) -> None:
        check("SELECT MAX(1, 2, 3)")

    def test_min_with_null(self) -> None:
        check("SELECT MIN(1, NULL)")

    def test_min_all_null(self) -> None:
        check("SELECT MIN(NULL, NULL)")

    def test_min_no_null(self) -> None:
        check("SELECT MIN(3, 1, 2)")


# ---------------------------------------------------------------------------
# 5. ABS on non-numeric text
# ---------------------------------------------------------------------------


class TestAbsNonNumeric:
    def test_abs_text_no_digits(self) -> None:
        check("SELECT ABS('hello')")

    def test_abs_text_leading_number(self) -> None:
        check("SELECT ABS('3.5abc')")

    def test_abs_null(self) -> None:
        check("SELECT ABS(NULL)")

    def test_abs_negative_float(self) -> None:
        check("SELECT ABS(-2.5)")


# ---------------------------------------------------------------------------
# 6. HEX(NULL)
# ---------------------------------------------------------------------------


class TestHexNull:
    def test_hex_null(self) -> None:
        check("SELECT HEX(NULL)")

    def test_hex_text(self) -> None:
        check("SELECT HEX('AB')")


# ---------------------------------------------------------------------------
# 7. DATE +1 month overflow into next month
# ---------------------------------------------------------------------------


class TestDateMonthOverflow:
    def test_jan31_plus_1_month_non_leap(self) -> None:
        # 2023 is not a leap year: Feb has 28 days.
        # Jan 31 + 1 month → Feb 31 → overflow → Mar 3
        check("SELECT date('2023-01-31', '+1 months')")

    def test_jan31_plus_1_month_leap(self) -> None:
        # 2024 is a leap year: Feb has 29 days.
        # Jan 31 + 1 month → Feb 31 → overflow → Mar 2
        check("SELECT date('2024-01-31', '+1 months')")

    def test_normal_month_no_overflow(self) -> None:
        # No overflow when the result day is valid.
        check("SELECT date('2024-01-15', '+1 months')")


# ---------------------------------------------------------------------------
# 8. LIKE NULL
# ---------------------------------------------------------------------------


class TestLikeNull:
    def test_like_null_pattern(self) -> None:
        """x LIKE NULL → NULL (not an error)."""
        check("SELECT 'hello' LIKE NULL")

    def test_null_like_pattern(self) -> None:
        """NULL LIKE '%' → NULL (normal NULL propagation)."""
        check("SELECT NULL LIKE '%'")
