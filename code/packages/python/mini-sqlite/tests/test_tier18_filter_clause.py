"""Tier 18 — FILTER (WHERE …) clause on aggregate functions.

SQLite (and SQL:2003) support a per-aggregate row predicate:

    COUNT(*) FILTER (WHERE active = 1)
    SUM(salary) FILTER (WHERE dept = 'eng')

Rows for which the FILTER expression evaluates to FALSE or NULL are silently
skipped before the accumulator is updated.  The clause is orthogonal to the
outer WHERE clause — both are applied: WHERE prunes rows from the scan, then
FILTER prunes rows from each individual aggregate.

Implementation notes:
  - The grammar was extended with a ``filter_clause`` rule attached to
    ``function_call``.
  - The planner carries ``filter_expr: Expr | None`` on ``AggregateExpr``
    and ``AggregateItem``.
  - The codegen emits ``JumpIfFalse(filter_skip)`` before the argument push
    and ``UpdateAgg``, keeping the operand stack balanced on both paths.
  - No VM changes: ``JumpIfFalse`` already exists.

All tests that do not rely on floating-point ordering of GROUP BY groups are
oracle-verified against real ``sqlite3``.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _both(sql: str, setup: list[str]) -> tuple[list, list]:
    """Run *sql* on both sqlite3 and mini_sqlite; return (ref, got)."""
    ref = sqlite3.connect(":memory:")
    got = mini_sqlite.connect(":memory:")
    for s in setup:
        ref.execute(s)
        got.execute(s)
    return ref.execute(sql).fetchall(), got.execute(sql).fetchall()


def _assert_both(sql: str, setup: list[str]) -> None:
    ref, got = _both(sql, setup)
    assert got == ref, f"mini-sqlite={got!r}, sqlite3={ref!r}\nSQL: {sql}"


EMP_SETUP = [
    "CREATE TABLE emp (name TEXT, dept TEXT, salary INTEGER, active INTEGER)",
    "INSERT INTO emp VALUES ('Alice', 'eng',   90000, 1)",
    "INSERT INTO emp VALUES ('Bob',   'sales', 80000, 1)",
    "INSERT INTO emp VALUES ('Carol', 'eng',   70000, 0)",
    "INSERT INTO emp VALUES ('Dave',  'sales', 65000, 0)",
    "INSERT INTO emp VALUES ('Eve',   'eng',   75000, 1)",
]


# ---------------------------------------------------------------------------
# COUNT(*) FILTER (WHERE …)
# ---------------------------------------------------------------------------


class TestCountStarFilter:
    """FILTER (WHERE …) on COUNT(*)."""

    def test_basic_filter(self) -> None:
        """Count only rows matching a simple predicate."""
        _assert_both(
            "SELECT COUNT(*) FILTER (WHERE dept = 'eng') FROM emp",
            EMP_SETUP,
        )

    def test_filter_matches_none(self) -> None:
        """When no rows match, result is 0 (COUNT never returns NULL)."""
        _assert_both(
            "SELECT COUNT(*) FILTER (WHERE dept = 'hr') FROM emp",
            EMP_SETUP,
        )

    def test_filter_matches_all(self) -> None:
        """When all rows match, result equals COUNT(*)."""
        _assert_both(
            "SELECT COUNT(*) FILTER (WHERE salary > 0) FROM emp",
            EMP_SETUP,
        )

    def test_multiple_filters_in_same_query(self) -> None:
        """Two FILTER aggregates in the same SELECT — independent accumulators."""
        _assert_both(
            "SELECT "
            "  COUNT(*) FILTER (WHERE dept = 'eng'), "
            "  COUNT(*) FILTER (WHERE dept = 'sales') "
            "FROM emp",
            EMP_SETUP,
        )

    def test_filter_with_group_by(self) -> None:
        """FILTER inside a GROUP BY query — applied per group."""
        _assert_both(
            "SELECT dept, COUNT(*) FILTER (WHERE active = 1) "
            "FROM emp GROUP BY dept ORDER BY dept",
            EMP_SETUP,
        )

    def test_filter_with_outer_where(self) -> None:
        """FILTER stacks with the outer WHERE clause."""
        _assert_both(
            "SELECT COUNT(*) FILTER (WHERE active = 1) "
            "FROM emp WHERE salary >= 70000",
            EMP_SETUP,
        )

    def test_filter_integer_boolean(self) -> None:
        """FILTER on an integer column (SQLite treats 0=false, non-zero=true)."""
        _assert_both(
            "SELECT COUNT(*) FILTER (WHERE active) FROM emp",
            EMP_SETUP,
        )


# ---------------------------------------------------------------------------
# SUM / AVG / MIN / MAX FILTER (WHERE …)
# ---------------------------------------------------------------------------


class TestSumAvgFilter:
    """FILTER on numeric aggregates."""

    def test_sum_filter(self) -> None:
        """SUM with FILTER sums only matching rows."""
        _assert_both(
            "SELECT SUM(salary) FILTER (WHERE dept = 'eng') FROM emp",
            EMP_SETUP,
        )

    def test_sum_filter_no_match_returns_null(self) -> None:
        """SUM with no matching rows returns NULL (SQLite behaviour)."""
        _assert_both(
            "SELECT SUM(salary) FILTER (WHERE dept = 'hr') FROM emp",
            EMP_SETUP,
        )

    def test_avg_filter(self) -> None:
        """AVG with FILTER averages only matching rows."""
        _assert_both(
            "SELECT AVG(salary) FILTER (WHERE active = 1) FROM emp",
            EMP_SETUP,
        )

    def test_avg_filter_no_match_returns_null(self) -> None:
        """AVG with no matching rows returns NULL."""
        _assert_both(
            "SELECT AVG(salary) FILTER (WHERE dept = 'hr') FROM emp",
            EMP_SETUP,
        )

    def test_min_filter(self) -> None:
        """MIN with FILTER finds the minimum among matching rows."""
        _assert_both(
            "SELECT MIN(salary) FILTER (WHERE dept = 'eng') FROM emp",
            EMP_SETUP,
        )

    def test_max_filter(self) -> None:
        """MAX with FILTER finds the maximum among matching rows."""
        _assert_both(
            "SELECT MAX(salary) FILTER (WHERE dept = 'sales') FROM emp",
            EMP_SETUP,
        )

    def test_sum_filter_with_group_by(self) -> None:
        """SUM FILTER per group: active salaries by department."""
        _assert_both(
            "SELECT dept, SUM(salary) FILTER (WHERE active = 1) "
            "FROM emp GROUP BY dept ORDER BY dept",
            EMP_SETUP,
        )

    def test_mixed_filter_and_plain_agg(self) -> None:
        """FILTER aggregate and plain aggregate in the same SELECT list."""
        _assert_both(
            "SELECT "
            "  SUM(salary), "
            "  SUM(salary) FILTER (WHERE active = 1) "
            "FROM emp",
            EMP_SETUP,
        )


# ---------------------------------------------------------------------------
# COUNT(col) FILTER — non-star COUNT
# ---------------------------------------------------------------------------


class TestCountColFilter:
    """FILTER on COUNT(col) — counts non-NULL values in matching rows."""

    def test_count_col_filter(self) -> None:
        """COUNT(col) FILTER counts non-NULL column values in matching rows."""
        _assert_both(
            "SELECT COUNT(name) FILTER (WHERE dept = 'eng') FROM emp",
            EMP_SETUP,
        )

    def test_count_col_filter_with_nulls(self) -> None:
        """NULL column values are not counted by COUNT(col) FILTER."""
        ref = sqlite3.connect(":memory:")
        got = mini_sqlite.connect(":memory:")
        for s in [
            "CREATE TABLE t (v TEXT, flag INTEGER)",
            "INSERT INTO t VALUES ('a', 1)",
            "INSERT INTO t VALUES (NULL, 1)",   # NULL value — not counted
            "INSERT INTO t VALUES ('b', 0)",    # flag=0 — filtered out
        ]:
            ref.execute(s)
            got.execute(s)
        ref_row = ref.execute("SELECT COUNT(v) FILTER (WHERE flag = 1) FROM t").fetchone()
        got_row = got.execute("SELECT COUNT(v) FILTER (WHERE flag = 1) FROM t").fetchone()
        assert got_row == ref_row == (1,)


# ---------------------------------------------------------------------------
# GROUP_CONCAT FILTER (WHERE …)
# ---------------------------------------------------------------------------


class TestGroupConcatFilter:
    """FILTER on GROUP_CONCAT."""

    def test_group_concat_filter(self) -> None:
        """GROUP_CONCAT FILTER concatenates only matching names."""
        # Ordering is insertion-order in SQLite and mini-sqlite, so direct
        # oracle comparison works.
        _assert_both(
            "SELECT GROUP_CONCAT(name) FILTER (WHERE dept = 'eng') FROM emp",
            EMP_SETUP,
        )

    def test_group_concat_filter_no_match(self) -> None:
        """GROUP_CONCAT FILTER with no matching rows returns NULL."""
        _assert_both(
            "SELECT GROUP_CONCAT(name) FILTER (WHERE dept = 'hr') FROM emp",
            EMP_SETUP,
        )


# ---------------------------------------------------------------------------
# FILTER with NULL predicate
# ---------------------------------------------------------------------------


class TestFilterNullPredicate:
    """Rows where FILTER expression is NULL are skipped (same as FALSE)."""

    def test_null_filter_skips_row(self) -> None:
        """NULL filter predicate = skip the row (matching SQLite NULL semantics)."""
        ref = sqlite3.connect(":memory:")
        got = mini_sqlite.connect(":memory:")
        for s in [
            "CREATE TABLE t (v INTEGER, flag INTEGER)",
            "INSERT INTO t VALUES (10, 1)",
            "INSERT INTO t VALUES (20, NULL)",  # NULL flag → skip
            "INSERT INTO t VALUES (30, 0)",     # 0 flag → skip
        ]:
            ref.execute(s)
            got.execute(s)
        ref_row = ref.execute("SELECT SUM(v) FILTER (WHERE flag) FROM t").fetchone()
        got_row = got.execute("SELECT SUM(v) FILTER (WHERE flag) FROM t").fetchone()
        assert got_row == ref_row == (10,)


# ---------------------------------------------------------------------------
# Edge cases
# ---------------------------------------------------------------------------


class TestFilterEdgeCases:
    """Edge-case behaviour verified against real sqlite3."""

    def test_filter_on_empty_table(self) -> None:
        """FILTER on an empty table: same as COUNT(*) on empty table = 0."""
        _assert_both(
            "SELECT COUNT(*) FILTER (WHERE 1) FROM emp",
            ["CREATE TABLE emp (x INTEGER)"],
        )

    def test_multiple_different_filters_per_group(self) -> None:
        """Three FILTER aggregates in the same GROUP BY query."""
        _assert_both(
            "SELECT dept, "
            "  COUNT(*) FILTER (WHERE active = 1), "
            "  COUNT(*) FILTER (WHERE active = 0), "
            "  COUNT(*) "
            "FROM emp GROUP BY dept ORDER BY dept",
            EMP_SETUP,
        )

    def test_filter_with_having(self) -> None:
        """FILTER clause combined with HAVING clause."""
        _assert_both(
            "SELECT dept, COUNT(*) FILTER (WHERE active = 1) AS active_cnt "
            "FROM emp GROUP BY dept HAVING active_cnt >= 2 ORDER BY dept",
            EMP_SETUP,
        )

    def test_filter_same_agg_different_filters(self) -> None:
        """Two SUM aggregates on the same column with different FILTER predicates."""
        _assert_both(
            "SELECT "
            "  SUM(salary) FILTER (WHERE active = 1), "
            "  SUM(salary) FILTER (WHERE active = 0) "
            "FROM emp",
            EMP_SETUP,
        )

    def test_filter_complex_predicate(self) -> None:
        """FILTER with a compound boolean predicate."""
        _assert_both(
            "SELECT COUNT(*) FILTER (WHERE dept = 'eng' AND active = 1) FROM emp",
            EMP_SETUP,
        )

    def test_filter_with_subquery_scalar(self) -> None:
        """FILTER predicate uses a scalar comparison (common pattern)."""
        _assert_both(
            "SELECT SUM(salary) FILTER (WHERE salary > 72000) FROM emp",
            EMP_SETUP,
        )
