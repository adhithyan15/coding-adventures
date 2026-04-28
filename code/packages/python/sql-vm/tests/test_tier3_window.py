"""
Window function VM tests — ComputeWindowFunctions instruction dispatch.

These are integration tests running the full pipeline (LogicalPlan →
codegen → VM) and verifying the ComputeWindowFunctions handler in vm.py.

Coverage targets
----------------
- vm.py: _do_compute_window_functions (lines 1087-1220)
- ir.py: ComputeWindowFunctions, WinFuncSpec, WinFunc
- All window function variants: ROW_NUMBER, RANK, DENSE_RANK, SUM, COUNT,
  COUNT_STAR, AVG, MIN, MAX, FIRST_VALUE, LAST_VALUE

Structure
---------
- TestRowNumber       — basic row numbering, global and partitioned
- TestRankFunctions   — RANK and DENSE_RANK with ties
- TestAggregateFuncs  — SUM, COUNT, COUNT_STAR, AVG, MIN, MAX
- TestValueFuncs      — FIRST_VALUE and LAST_VALUE
- TestPartitionBy     — partition_by semantics
- TestOrderBy         — order_by within partitions
"""

from __future__ import annotations

from sql_backend.in_memory import InMemoryBackend
from sql_backend.schema import ColumnDef
from sql_codegen import (
    ComputeWindowFunctions,
    WinFunc,
    compile,
)
from sql_planner import (
    Column,
    Project,
    ProjectionItem,
    Scan,
    WindowAgg,
)
from sql_planner.plan import WindowFuncSpec as PlanWindowFuncSpec

from sql_vm import execute

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _backend_with_employees() -> InMemoryBackend:
    be = InMemoryBackend()
    be.create_table(
        "employees",
        [
            ColumnDef(name="id", type_name="INTEGER", primary_key=True),
            ColumnDef(name="name", type_name="TEXT"),
            ColumnDef(name="dept", type_name="TEXT"),
            ColumnDef(name="salary", type_name="INTEGER"),
        ],
        False,
    )
    rows = [
        {"id": 1, "name": "Alice", "dept": "eng", "salary": 90000},
        {"id": 2, "name": "Bob", "dept": "eng", "salary": 80000},
        {"id": 3, "name": "Carol", "dept": "sales", "salary": 70000},
        {"id": 4, "name": "Dave", "dept": "sales", "salary": 75000},
        {"id": 5, "name": "Eve", "dept": "eng", "salary": 85000},
    ]
    for r in rows:
        be.insert("employees", r)
    return be


def _run_window(
    specs: list[PlanWindowFuncSpec],
    select_cols: list[str],
    output_cols: list[str],
) -> list[tuple]:
    """Build a WindowAgg plan, compile, execute, and return rows."""
    be = _backend_with_employees()
    col_items = tuple(
        ProjectionItem(expr=Column(table="e", col=c), alias=None)
        for c in select_cols
    )
    inner = Project(
        input=Scan(table="employees", alias="e"),
        items=col_items,
    )
    plan = WindowAgg(
        input=inner,
        specs=tuple(specs),
        output_cols=tuple(output_cols),
    )
    prog = compile(plan)
    result = execute(prog, be)
    return result.rows


# ---------------------------------------------------------------------------
# ROW_NUMBER
# ---------------------------------------------------------------------------


class TestRowNumber:
    def test_global_row_number(self) -> None:
        spec = PlanWindowFuncSpec(
            func="row_number",
            arg_expr=None,
            partition_by=(),
            order_by=(),
            alias="rn",
        )
        rows = _run_window([spec], ["id", "name"], ["id", "name", "rn"])
        assert len(rows) == 5
        rns = sorted(r[2] for r in rows)
        assert rns == [1, 2, 3, 4, 5]

    def test_row_number_values_unique(self) -> None:
        spec = PlanWindowFuncSpec(
            func="row_number",
            arg_expr=None,
            partition_by=(),
            order_by=(),
            alias="rn",
        )
        rows = _run_window([spec], ["id"], ["id", "rn"])
        rn_vals = [r[1] for r in rows]
        assert len(set(rn_vals)) == 5

    def test_row_number_in_partition(self) -> None:
        spec = PlanWindowFuncSpec(
            func="row_number",
            arg_expr=None,
            partition_by=(Column(table="employees", col="dept"),),
            order_by=(),
            alias="rn",
        )
        rows = _run_window([spec], ["dept", "name"], ["dept", "name", "rn"])
        eng_rows = sorted(r[2] for r in rows if r[0] == "eng")
        sales_rows = sorted(r[2] for r in rows if r[0] == "sales")
        assert eng_rows == [1, 2, 3]
        assert sales_rows == [1, 2]


# ---------------------------------------------------------------------------
# RANK and DENSE_RANK
# ---------------------------------------------------------------------------


class TestRankFunctions:
    def test_rank_with_ties(self) -> None:
        """Two rows with the same order_by value get the same rank."""
        spec = PlanWindowFuncSpec(
            func="rank",
            arg_expr=None,
            partition_by=(),
            order_by=((Column(table="employees", col="dept"), False),),
            alias="r",
        )
        rows = _run_window([spec], ["dept", "name"], ["dept", "name", "r"])
        # eng (3 rows) all tie for rank 1; sales (2 rows) tie for rank 4
        eng_ranks = {r[2] for r in rows if r[0] == "eng"}
        sales_ranks = {r[2] for r in rows if r[0] == "sales"}
        assert eng_ranks == {1}
        assert sales_ranks == {4}

    def test_dense_rank_no_gaps(self) -> None:
        """DENSE_RANK produces consecutive values even with ties."""
        spec = PlanWindowFuncSpec(
            func="dense_rank",
            arg_expr=None,
            partition_by=(),
            order_by=((Column(table="employees", col="dept"), False),),
            alias="dr",
        )
        rows = _run_window([spec], ["dept", "name"], ["dept", "name", "dr"])
        ranks = sorted({r[2] for r in rows})
        assert ranks == [1, 2]

    def test_rank_in_partition_by_salary(self) -> None:
        """RANK within partition ordered by salary desc."""
        spec = PlanWindowFuncSpec(
            func="rank",
            arg_expr=None,
            partition_by=(Column(table="employees", col="dept"),),
            order_by=((Column(table="employees", col="salary"), True),),
            alias="r",
        )
        rows = _run_window([spec], ["dept", "name", "salary"], ["dept", "name", "salary", "r"])
        # In eng: Alice 90000→rank1, Eve 85000→rank2, Bob 80000→rank3
        eng_by_salary = sorted(
            [(r[2], r[3]) for r in rows if r[0] == "eng"],
            key=lambda x: -x[0],
        )
        assert eng_by_salary[0] == (90000, 1)
        assert eng_by_salary[1] == (85000, 2)
        assert eng_by_salary[2] == (80000, 3)


# ---------------------------------------------------------------------------
# Aggregate window functions
# ---------------------------------------------------------------------------


class TestAggregateFuncs:
    def test_sum_global(self) -> None:
        spec = PlanWindowFuncSpec(
            func="sum",
            arg_expr=Column(table="employees", col="salary"),
            partition_by=(),
            order_by=(),
            alias="total",
        )
        rows = _run_window([spec], ["name", "salary"], ["name", "salary", "total"])
        totals = {r[2] for r in rows}
        assert totals == {400000}  # 90000+80000+70000+75000+85000

    def test_sum_by_partition(self) -> None:
        spec = PlanWindowFuncSpec(
            func="sum",
            arg_expr=Column(table="employees", col="salary"),
            partition_by=(Column(table="employees", col="dept"),),
            order_by=(),
            alias="dept_total",
        )
        rows = _run_window([spec], ["dept", "salary"], ["dept", "salary", "dept_total"])
        eng_totals = {r[2] for r in rows if r[0] == "eng"}
        sales_totals = {r[2] for r in rows if r[0] == "sales"}
        assert eng_totals == {255000}   # 90000+80000+85000
        assert sales_totals == {145000}  # 70000+75000

    def test_count_star(self) -> None:
        spec = PlanWindowFuncSpec(
            func="count_star",
            arg_expr=None,
            partition_by=(Column(table="employees", col="dept"),),
            order_by=(),
            alias="dept_count",
        )
        rows = _run_window([spec], ["dept"], ["dept", "dept_count"])
        eng_count = {r[1] for r in rows if r[0] == "eng"}
        sales_count = {r[1] for r in rows if r[0] == "sales"}
        assert eng_count == {3}
        assert sales_count == {2}

    def test_count_non_null(self) -> None:
        spec = PlanWindowFuncSpec(
            func="count",
            arg_expr=Column(table="employees", col="salary"),
            partition_by=(Column(table="employees", col="dept"),),
            order_by=(),
            alias="cnt",
        )
        rows = _run_window([spec], ["dept", "salary"], ["dept", "salary", "cnt"])
        eng_cnt = {r[2] for r in rows if r[0] == "eng"}
        assert eng_cnt == {3}

    def test_avg_by_partition(self) -> None:
        spec = PlanWindowFuncSpec(
            func="avg",
            arg_expr=Column(table="employees", col="salary"),
            partition_by=(Column(table="employees", col="dept"),),
            order_by=(),
            alias="avg_sal",
        )
        rows = _run_window([spec], ["dept", "salary"], ["dept", "salary", "avg_sal"])
        eng_avgs = {r[2] for r in rows if r[0] == "eng"}
        assert len(eng_avgs) == 1
        assert abs(list(eng_avgs)[0] - 85000.0) < 0.01

    def test_min_by_partition(self) -> None:
        spec = PlanWindowFuncSpec(
            func="min",
            arg_expr=Column(table="employees", col="salary"),
            partition_by=(Column(table="employees", col="dept"),),
            order_by=(),
            alias="min_sal",
        )
        rows = _run_window([spec], ["dept", "salary"], ["dept", "salary", "min_sal"])
        eng_mins = {r[2] for r in rows if r[0] == "eng"}
        assert eng_mins == {80000}

    def test_max_by_partition(self) -> None:
        spec = PlanWindowFuncSpec(
            func="max",
            arg_expr=Column(table="employees", col="salary"),
            partition_by=(Column(table="employees", col="dept"),),
            order_by=(),
            alias="max_sal",
        )
        rows = _run_window([spec], ["dept", "salary"], ["dept", "salary", "max_sal"])
        eng_maxes = {r[2] for r in rows if r[0] == "eng"}
        assert eng_maxes == {90000}


# ---------------------------------------------------------------------------
# FIRST_VALUE and LAST_VALUE
# ---------------------------------------------------------------------------


class TestValueFuncs:
    def test_first_value(self) -> None:
        spec = PlanWindowFuncSpec(
            func="first_value",
            arg_expr=Column(table="employees", col="name"),
            partition_by=(Column(table="employees", col="dept"),),
            order_by=((Column(table="employees", col="salary"), True),),
            alias="first_name",
        )
        rows = _run_window(
            [spec], ["dept", "name", "salary"],
            ["dept", "name", "salary", "first_name"],
        )
        # In eng sorted by salary desc: Alice(90000) is first
        eng_first = {r[3] for r in rows if r[0] == "eng"}
        assert eng_first == {"Alice"}

    def test_last_value(self) -> None:
        spec = PlanWindowFuncSpec(
            func="last_value",
            arg_expr=Column(table="employees", col="name"),
            partition_by=(Column(table="employees", col="dept"),),
            order_by=((Column(table="employees", col="salary"), True),),
            alias="last_name",
        )
        rows = _run_window(
            [spec], ["dept", "name", "salary"],
            ["dept", "name", "salary", "last_name"],
        )
        # In eng sorted by salary desc: Bob(80000) is last
        eng_last = {r[3] for r in rows if r[0] == "eng"}
        assert eng_last == {"Bob"}


# ---------------------------------------------------------------------------
# Multiple window functions in one WindowAgg node
# ---------------------------------------------------------------------------


class TestMultipleWindowFuncs:
    def test_two_window_funcs_same_node(self) -> None:
        spec_rn = PlanWindowFuncSpec(
            func="row_number",
            arg_expr=None,
            partition_by=(Column(table="employees", col="dept"),),
            order_by=(),
            alias="rn",
        )
        spec_sum = PlanWindowFuncSpec(
            func="sum",
            arg_expr=Column(table="employees", col="salary"),
            partition_by=(Column(table="employees", col="dept"),),
            order_by=(),
            alias="dept_total",
        )
        rows = _run_window(
            [spec_rn, spec_sum],
            ["dept", "name", "salary"],
            ["dept", "name", "salary", "rn", "dept_total"],
        )
        assert len(rows) == 5
        # Check that both columns are populated
        rn_vals = [r[3] for r in rows]
        total_vals = [r[4] for r in rows]
        assert all(v is not None for v in rn_vals)
        assert all(v is not None for v in total_vals)

    def test_output_cols_order_preserved(self) -> None:
        """WindowAgg respects output_cols ordering."""
        spec = PlanWindowFuncSpec(
            func="row_number",
            arg_expr=None,
            partition_by=(),
            order_by=(),
            alias="rn",
        )
        rows = _run_window([spec], ["name", "dept"], ["name", "dept", "rn"])
        assert len(rows) == 5
        assert all(len(r) == 3 for r in rows)


# ---------------------------------------------------------------------------
# ComputeWindowFunctions IR instruction tests (direct compile output checks)
# ---------------------------------------------------------------------------


class TestComputeWindowFunctionsIR:
    def test_compile_produces_compute_window_functions(self) -> None:
        """Compiling a WindowAgg plan produces a ComputeWindowFunctions instruction."""
        spec = PlanWindowFuncSpec(
            func="row_number",
            arg_expr=None,
            partition_by=(),
            order_by=(),
            alias="rn",
        )
        inner = Project(
            input=Scan(table="employees", alias="e"),
            items=(ProjectionItem(expr=Column(table="e", col="id"), alias=None),),
        )
        plan = WindowAgg(input=inner, specs=(spec,), output_cols=("id", "rn"))
        prog = compile(plan)
        assert any(isinstance(ins, ComputeWindowFunctions) for ins in prog.instructions)

    def test_win_func_spec_has_correct_func(self) -> None:
        """WinFuncSpec in the program has the correct WinFunc enum value."""
        spec = PlanWindowFuncSpec(
            func="sum",
            arg_expr=Column(table="e", col="salary"),
            partition_by=(),
            order_by=(),
            alias="total",
        )
        inner = Project(
            input=Scan(table="employees", alias="e"),
            items=(ProjectionItem(expr=Column(table="e", col="salary"), alias=None),),
        )
        plan = WindowAgg(input=inner, specs=(spec,), output_cols=("salary", "total"))
        prog = compile(plan)
        cwf = next(ins for ins in prog.instructions if isinstance(ins, ComputeWindowFunctions))
        assert cwf.specs[0].func == WinFunc.SUM
