"""
Tier-3 integration tests — Window functions (OVER / PARTITION BY)
=================================================================

Tests cover:
  1. Grammar    — the parser can produce window_func_call nodes
  2. Adapter    — correct translation to WindowFuncExpr
  3. Planner    — WindowAgg plan node structure
  4. End-to-end — window functions via mini_sqlite.connect(":memory:")

Functions exercised:
  ROW_NUMBER, RANK, DENSE_RANK, SUM, COUNT, COUNT(*), AVG, MIN, MAX,
  FIRST_VALUE, LAST_VALUE

Window spec combinations:
  - No PARTITION, no ORDER (global window)
  - PARTITION BY only
  - ORDER BY only
  - PARTITION BY + ORDER BY

The test structure mirrors test_tier3_savepoint.py.
"""

from __future__ import annotations

from sql_parser import parse_sql
from sql_planner import WindowAgg, WindowFuncExpr

import mini_sqlite
from mini_sqlite.adapter import to_statement

# ---------------------------------------------------------------------------
# Helpers.
# ---------------------------------------------------------------------------


def _parse(sql: str):
    return parse_sql(sql)


def _stmt(sql: str):
    return to_statement(_parse(sql))


def _conn():
    """Return a fresh in-memory connection with an employees table."""
    con = mini_sqlite.connect(":memory:")
    con.execute("CREATE TABLE employees (id INTEGER, name TEXT, dept TEXT, salary INTEGER)")
    con.executemany(
        "INSERT INTO employees VALUES (?, ?, ?, ?)",
        [
            (1, "Alice", "Engineering", 90000),
            (2, "Bob",   "Engineering", 80000),
            (3, "Carol", "Marketing",   70000),
            (4, "Dave",  "Marketing",   70000),
            (5, "Eve",   "HR",          60000),
        ],
    )
    return con


def _rows(con, sql: str) -> list[tuple]:
    cur = con.execute(sql)
    return cur.fetchall()


# ===========================================================================
# 1. Grammar tests
# ===========================================================================


class TestWindowGrammar:
    def test_row_number_parses(self):
        ast = _parse("SELECT ROW_NUMBER() OVER () FROM t")
        stmt_node = ast.children[0]
        # Reaches here without ParseError → grammar accepted window_func_call.
        assert stmt_node is not None

    def test_partition_by_parses(self):
        ast = _parse("SELECT ROW_NUMBER() OVER (PARTITION BY dept) FROM t")
        assert ast is not None

    def test_order_by_parses(self):
        ast = _parse("SELECT RANK() OVER (ORDER BY salary DESC) FROM t")
        assert ast is not None

    def test_partition_and_order_parses(self):
        ast = _parse(
            "SELECT SUM(salary) OVER (PARTITION BY dept ORDER BY salary) FROM t"
        )
        assert ast is not None

    def test_count_star_parses(self):
        ast = _parse("SELECT COUNT(*) OVER (PARTITION BY dept) FROM t")
        assert ast is not None

    def test_window_func_call_rule_name(self):
        """The parser should produce a window_func_call node, not function_call."""
        ast = _parse("SELECT ROW_NUMBER() OVER () FROM t")
        # Walk into: program → statement → query_stmt → select_stmt →
        # select_list → select_item → expr → ... → window_func_call
        found = False

        def walk(node):
            nonlocal found
            if hasattr(node, "rule_name") and node.rule_name == "window_func_call":
                found = True
            if hasattr(node, "children"):
                for c in node.children:
                    walk(c)

        walk(ast)
        assert found, "Expected a window_func_call node in the parse tree"

    def test_function_call_not_window_without_over(self):
        """SUM(x) without OVER should still parse as function_call, not window."""
        ast = _parse("SELECT SUM(salary) FROM t GROUP BY dept")
        found_window = False

        def walk(node):
            nonlocal found_window
            if hasattr(node, "rule_name") and node.rule_name == "window_func_call":
                found_window = True
            if hasattr(node, "children"):
                for c in node.children:
                    walk(c)

        walk(ast)
        assert not found_window, "SUM without OVER should not produce a window_func_call node"


# ===========================================================================
# 2. Adapter tests (parse tree → WindowFuncExpr)
# ===========================================================================


class TestWindowAdapter:
    def _expr_from_sql(self, sql: str):
        """Extract the first expression from the SELECT list."""
        from sql_planner import SelectStmt

        from mini_sqlite.adapter import to_statement

        stmt = to_statement(parse_sql(sql))
        assert isinstance(stmt, SelectStmt)
        return stmt.items[0].expr

    def test_row_number_no_spec(self):
        expr = self._expr_from_sql("SELECT ROW_NUMBER() OVER () FROM t")
        assert isinstance(expr, WindowFuncExpr)
        assert expr.func == "row_number"
        assert expr.arg is None
        assert expr.partition_by == ()
        assert expr.order_by == ()

    def test_rank_with_order(self):
        expr = self._expr_from_sql("SELECT RANK() OVER (ORDER BY salary) FROM t")
        assert isinstance(expr, WindowFuncExpr)
        assert expr.func == "rank"
        assert expr.arg is None
        assert len(expr.order_by) == 1
        col_expr, desc = expr.order_by[0]
        assert desc is False

    def test_dense_rank(self):
        expr = self._expr_from_sql("SELECT DENSE_RANK() OVER (ORDER BY salary DESC) FROM t")
        assert isinstance(expr, WindowFuncExpr)
        assert expr.func == "dense_rank"
        _, desc = expr.order_by[0]
        assert desc is True

    def test_sum_with_partition(self):
        expr = self._expr_from_sql(
            "SELECT SUM(salary) OVER (PARTITION BY dept) FROM t"
        )
        assert isinstance(expr, WindowFuncExpr)
        assert expr.func == "sum"
        assert expr.arg is not None
        assert len(expr.partition_by) == 1

    def test_count_star_becomes_count_star_func(self):
        expr = self._expr_from_sql(
            "SELECT COUNT(*) OVER (PARTITION BY dept) FROM t"
        )
        assert isinstance(expr, WindowFuncExpr)
        assert expr.func == "count_star"
        assert expr.arg is None

    def test_count_col(self):
        expr = self._expr_from_sql(
            "SELECT COUNT(salary) OVER (PARTITION BY dept) FROM t"
        )
        assert isinstance(expr, WindowFuncExpr)
        assert expr.func == "count"
        assert expr.arg is not None

    def test_avg(self):
        expr = self._expr_from_sql(
            "SELECT AVG(salary) OVER (PARTITION BY dept) FROM t"
        )
        assert isinstance(expr, WindowFuncExpr)
        assert expr.func == "avg"

    def test_min_max(self):
        min_expr = self._expr_from_sql("SELECT MIN(salary) OVER () FROM t")
        max_expr = self._expr_from_sql("SELECT MAX(salary) OVER () FROM t")
        assert isinstance(min_expr, WindowFuncExpr)
        assert min_expr.func == "min"
        assert isinstance(max_expr, WindowFuncExpr)
        assert max_expr.func == "max"

    def test_first_value(self):
        expr = self._expr_from_sql(
            "SELECT FIRST_VALUE(salary) OVER (ORDER BY salary) FROM t"
        )
        assert isinstance(expr, WindowFuncExpr)
        assert expr.func == "first_value"
        assert expr.arg is not None

    def test_last_value(self):
        expr = self._expr_from_sql(
            "SELECT LAST_VALUE(salary) OVER (ORDER BY salary) FROM t"
        )
        assert isinstance(expr, WindowFuncExpr)
        assert expr.func == "last_value"

    def test_partition_and_order(self):
        expr = self._expr_from_sql(
            "SELECT SUM(salary) OVER (PARTITION BY dept ORDER BY id) FROM t"
        )
        assert isinstance(expr, WindowFuncExpr)
        assert len(expr.partition_by) == 1
        assert len(expr.order_by) == 1

    def test_alias_preserved(self):
        from sql_planner import SelectStmt

        stmt = to_statement(parse_sql(
            "SELECT ROW_NUMBER() OVER () AS rn FROM t"
        ))
        assert isinstance(stmt, SelectStmt)
        item = stmt.items[0]
        assert item.alias == "rn"


# ===========================================================================
# 3. Planner tests (WindowFuncExpr → WindowAgg plan node)
# ===========================================================================


class TestWindowPlanner:
    def _plan(self, sql: str) -> object:
        from sql_planner import InMemorySchemaProvider, plan

        from mini_sqlite.adapter import to_statement

        schema = InMemorySchemaProvider({
            "employees": ["id", "name", "dept", "salary"],
        })
        stmt = to_statement(parse_sql(sql))
        return plan(stmt, schema)

    def test_produces_window_agg(self):
        from sql_planner import Distinct, Limit, Sort

        p = self._plan(
            "SELECT ROW_NUMBER() OVER () AS rn FROM employees"
        )
        # Unwrap any outer decorators (Sort/Limit/Distinct).
        while isinstance(p, (Sort, Limit, Distinct)):
            p = p.input
        assert isinstance(p, WindowAgg), f"Expected WindowAgg, got {type(p).__name__}"

    def test_output_cols_includes_alias(self):
        from sql_planner import Distinct, Limit, Sort

        p = self._plan(
            "SELECT name, ROW_NUMBER() OVER () AS rn FROM employees"
        )
        while isinstance(p, (Sort, Limit, Distinct)):
            p = p.input
        assert isinstance(p, WindowAgg)
        assert "rn" in p.output_cols
        assert "name" in p.output_cols

    def test_inner_plan_is_project(self):
        from sql_planner import Distinct, Limit, Project, Sort

        p = self._plan(
            "SELECT SUM(salary) OVER (PARTITION BY dept) AS dept_total FROM employees"
        )
        while isinstance(p, (Sort, Limit, Distinct)):
            p = p.input
        assert isinstance(p, WindowAgg)
        assert isinstance(p.input, Project)

    def test_window_spec_func_name(self):
        from sql_planner import Distinct, Limit, Sort, WindowFuncSpec

        p = self._plan(
            "SELECT DENSE_RANK() OVER (ORDER BY salary DESC) AS dr FROM employees"
        )
        while isinstance(p, (Sort, Limit, Distinct)):
            p = p.input
        assert isinstance(p, WindowAgg)
        spec = p.specs[0]
        assert isinstance(spec, WindowFuncSpec)
        assert spec.func == "dense_rank"

    def test_order_by_above_window_agg(self):
        """ORDER BY in the outer SELECT should still produce a Sort wrapper."""
        from sql_planner import Sort

        p = self._plan(
            "SELECT name, ROW_NUMBER() OVER () AS rn FROM employees ORDER BY name"
        )
        assert isinstance(p, Sort), "Expected outer Sort wrapping WindowAgg"


# ===========================================================================
# 4. End-to-end integration tests
# ===========================================================================


class TestWindowIntegration:
    def test_row_number_global(self):
        """ROW_NUMBER() OVER () numbers all rows 1..N in output order."""
        con = _conn()
        rows = _rows(con, "SELECT id, ROW_NUMBER() OVER () AS rn FROM employees ORDER BY id")
        rn_values = [r[1] for r in rows]
        assert sorted(rn_values) == list(range(1, 6))

    def test_row_number_partitioned(self):
        """ROW_NUMBER() resets to 1 for each department partition."""
        con = _conn()
        rows = _rows(
            con,
            "SELECT dept, ROW_NUMBER() OVER (PARTITION BY dept ORDER BY salary DESC) AS rn "
            "FROM employees ORDER BY dept",
        )
        # Engineering (2 rows → rn 1, 2), HR (1 row → rn 1), Marketing (2 rows → rn 1, 2)
        by_dept: dict[str, list[int]] = {}
        for dept, rn in rows:
            by_dept.setdefault(dept, []).append(rn)
        assert 1 in by_dept["Engineering"]
        assert 2 in by_dept["Engineering"]
        assert by_dept["HR"] == [1]
        assert 1 in by_dept["Marketing"]
        assert 2 in by_dept["Marketing"]

    def test_rank_with_ties(self):
        """RANK() assigns the same rank to tied rows, skipping the next rank."""
        con = _conn()
        rows = _rows(
            con,
            "SELECT salary, RANK() OVER (ORDER BY salary DESC) AS r "
            "FROM employees ORDER BY salary DESC",
        )
        ranks = [r[1] for r in rows]
        # 90000→1, 80000→2, 70000→3 (two rows), 70000→3, 60000→5
        assert ranks[0] == 1   # Alice 90000
        assert ranks[1] == 2   # Bob 80000
        assert ranks[2] == 3   # Carol or Dave 70000
        assert ranks[3] == 3   # Carol or Dave 70000
        assert ranks[4] == 5   # Eve 60000

    def test_dense_rank_with_ties(self):
        """DENSE_RANK() assigns same rank to ties without skipping."""
        con = _conn()
        rows = _rows(
            con,
            "SELECT salary, DENSE_RANK() OVER (ORDER BY salary DESC) AS dr "
            "FROM employees ORDER BY salary DESC",
        )
        ranks = [r[1] for r in rows]
        assert ranks[0] == 1
        assert ranks[1] == 2
        assert ranks[2] == 3
        assert ranks[3] == 3   # tie, same dense_rank
        assert ranks[4] == 4   # next rank is 4, not 5

    def test_sum_partition(self):
        """SUM(salary) OVER (PARTITION BY dept) gives the dept total for every row."""
        con = _conn()
        rows = _rows(
            con,
            "SELECT dept, salary, SUM(salary) OVER (PARTITION BY dept) AS dept_total "
            "FROM employees ORDER BY dept, salary",
        )
        dept_totals = {dept: total for dept, _, total in rows}
        assert dept_totals["Engineering"] == 90000 + 80000
        assert dept_totals["Marketing"] == 70000 + 70000
        assert dept_totals["HR"] == 60000

    def test_count_star_global(self):
        """COUNT(*) OVER () is the total row count for every row."""
        con = _conn()
        rows = _rows(con, "SELECT id, COUNT(*) OVER () AS cnt FROM employees ORDER BY id")
        counts = {r[0]: r[1] for r in rows}
        assert all(v == 5 for v in counts.values())

    def test_count_col_partition(self):
        """COUNT(col) OVER (PARTITION BY dept) counts non-NULL values per dept."""
        con = _conn()
        rows = _rows(
            con,
            "SELECT dept, COUNT(salary) OVER (PARTITION BY dept) AS cnt "
            "FROM employees ORDER BY dept",
        )
        counts = {dept: cnt for dept, cnt in rows}
        assert counts["Engineering"] == 2
        assert counts["Marketing"] == 2
        assert counts["HR"] == 1

    def test_avg_partition(self):
        """AVG(salary) OVER (PARTITION BY dept) gives the dept average."""
        con = _conn()
        rows = _rows(
            con,
            "SELECT dept, AVG(salary) OVER (PARTITION BY dept) AS avg_sal "
            "FROM employees ORDER BY dept",
        )
        avgs = {dept: avg for dept, avg in rows}
        assert abs(avgs["Engineering"] - 85000.0) < 0.001
        assert abs(avgs["Marketing"] - 70000.0) < 0.001
        assert abs(avgs["HR"] - 60000.0) < 0.001

    def test_min_max_global(self):
        """MIN/MAX OVER () return the global min/max for every row."""
        con = _conn()
        rows = _rows(
            con,
            "SELECT MIN(salary) OVER () AS mn, MAX(salary) OVER () AS mx "
            "FROM employees LIMIT 1",
        )
        mn, mx = rows[0]
        assert mn == 60000
        assert mx == 90000

    def test_first_last_value(self):
        """FIRST_VALUE and LAST_VALUE over ordered partition."""
        con = _conn()
        rows = _rows(
            con,
            "SELECT dept, "
            "FIRST_VALUE(salary) OVER (PARTITION BY dept ORDER BY salary DESC) AS highest, "
            "LAST_VALUE(salary) OVER (PARTITION BY dept ORDER BY salary DESC) AS lowest "
            "FROM employees ORDER BY dept",
        )
        eng = [(dept, hi, lo) for dept, hi, lo in rows if dept == "Engineering"]
        # Highest salary in Engineering = 90000, lowest = 80000
        assert all(hi == 90000 for _, hi, lo in eng)
        assert all(lo == 80000 for _, hi, lo in eng)

    def test_multiple_window_functions(self):
        """Multiple window functions in one SELECT are all computed."""
        con = _conn()
        rows = _rows(
            con,
            "SELECT name, "
            "ROW_NUMBER() OVER (ORDER BY salary DESC) AS rn, "
            "DENSE_RANK() OVER (ORDER BY salary DESC) AS dr "
            "FROM employees ORDER BY name",
        )
        assert len(rows) == 5
        # Alice (90000) → rn=1, dr=1
        alice = next(r for r in rows if r[0] == "Alice")
        assert alice[1] == 1
        assert alice[2] == 1

    def test_window_with_where_clause(self):
        """WHERE filters rows before window computation."""
        con = _conn()
        rows = _rows(
            con,
            "SELECT name, ROW_NUMBER() OVER (ORDER BY salary DESC) AS rn "
            "FROM employees WHERE dept = 'Engineering' ORDER BY name",
        )
        # Engineering: Alice (90000, rn=1) and Bob (80000, rn=2).
        # ORDER BY name → Alice first, Bob second.
        assert len(rows) == 2
        assert rows[0] == ("Alice", 1)
        assert rows[1] == ("Bob", 2)

    def test_window_preserves_non_window_columns(self):
        """Non-window SELECT items appear correctly in the output."""
        con = _conn()
        rows = _rows(
            con,
            "SELECT name, dept, ROW_NUMBER() OVER () AS rn FROM employees ORDER BY name",
        )
        assert len(rows) == 5
        alice = next(r for r in rows if r[0] == "Alice")
        assert alice[1] == "Engineering"

    def test_window_null_handling(self):
        """Rows with NULL values in partition/order columns are handled."""
        con = mini_sqlite.connect(":memory:")
        con.execute("CREATE TABLE t (x INTEGER, y INTEGER)")
        con.executemany("INSERT INTO t VALUES (?, ?)", [
            (1, None),
            (2, 10),
            (3, None),
            (4, 20),
        ])
        rows = _rows(con, "SELECT x, ROW_NUMBER() OVER (ORDER BY y) AS rn FROM t ORDER BY x")
        # NULLs sort first in SQLite-compatible ordering
        rn_vals = [r[1] for r in rows]
        assert sorted(rn_vals) == [1, 2, 3, 4]

    def test_sum_null_ignores_nulls(self):
        """SUM skips NULL values, matching SQL standard aggregate behaviour."""
        con = mini_sqlite.connect(":memory:")
        con.execute("CREATE TABLE t (dept TEXT, salary INTEGER)")
        con.executemany("INSERT INTO t VALUES (?, ?)", [
            ("A", 100),
            ("A", None),
            ("A", 200),
        ])
        rows = _rows(con, "SELECT SUM(salary) OVER (PARTITION BY dept) FROM t LIMIT 1")
        assert rows[0][0] == 300

    def test_row_number_alias_in_result_columns(self):
        """The window function alias appears as the column name in the cursor."""
        con = _conn()
        cur = con.execute("SELECT ROW_NUMBER() OVER () AS row_num FROM employees LIMIT 1")
        # description[0][0] should be "row_num"
        assert cur.description[0][0] == "row_num"

    def test_window_with_order_by_limit(self):
        """ORDER BY and LIMIT above a WindowAgg work correctly."""
        con = _conn()
        rows = _rows(
            con,
            "SELECT name, ROW_NUMBER() OVER () AS rn FROM employees ORDER BY name LIMIT 3",
        )
        assert len(rows) == 3
        names = [r[0] for r in rows]
        assert names == sorted(names)
