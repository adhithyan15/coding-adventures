"""
Extended window function VM tests — LAG, LEAD, NTILE, PERCENT_RANK, CUME_DIST, NTH_VALUE.

These are integration tests running the full pipeline (LogicalPlan →
codegen → VM) and verifying the new window function branches added to
``_do_compute_window`` in vm.py.

Data set (re-used throughout)
------------------------------
Five employees in two departments, ordered here for reference:

    id  name   dept   salary
    1   Alice  eng    90000
    2   Bob    eng    80000
    3   Carol  sales  70000
    4   Dave   sales  75000
    5   Eve    eng    85000

When sorted by salary ASC within each dept:

    eng:   Bob 80000, Eve 85000, Alice 90000
    sales: Carol 70000, Dave 75000

Coverage targets
----------------
- vm.py: WinFunc.LAG, WinFunc.LEAD, WinFunc.NTILE,
         WinFunc.PERCENT_RANK, WinFunc.CUME_DIST, WinFunc.NTH_VALUE
"""

from __future__ import annotations

from sql_backend.in_memory import InMemoryBackend
from sql_backend.schema import ColumnDef
from sql_codegen import compile
from sql_planner import (
    Column,
    Literal,
    Project,
    ProjectionItem,
    Scan,
    WindowAgg,
)
from sql_planner.plan import WindowFuncSpec as PlanWindowFuncSpec

from sql_vm import execute

# ---------------------------------------------------------------------------
# Shared helpers
# ---------------------------------------------------------------------------


def _backend() -> InMemoryBackend:
    """Create an in-memory backend pre-loaded with the five-row employees table."""
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
    for row in [
        {"id": 1, "name": "Alice", "dept": "eng",   "salary": 90000},
        {"id": 2, "name": "Bob",   "dept": "eng",   "salary": 80000},
        {"id": 3, "name": "Carol", "dept": "sales", "salary": 70000},
        {"id": 4, "name": "Dave",  "dept": "sales", "salary": 75000},
        {"id": 5, "name": "Eve",   "dept": "eng",   "salary": 85000},
    ]:
        be.insert("employees", row)
    return be


def _run(
    specs: list[PlanWindowFuncSpec],
    select_cols: list[str],
    output_cols: list[str],
) -> list[tuple]:
    """Build a WindowAgg plan, compile, execute, and return result rows."""
    be = _backend()
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
    return execute(prog, be).rows


# Shorthand column factory (table="employees" matches the Scan alias scope used
# in PlanWindowFuncSpec partition_by / order_by).
def _col(name: str) -> Column:
    return Column(table="employees", col=name)


# ---------------------------------------------------------------------------
# LAG
# ---------------------------------------------------------------------------


class TestLag:
    """LAG(col [, offset=1 [, default=NULL]]) looks back in an ordered partition."""

    def test_lag_default_offset(self) -> None:
        """LAG with no explicit offset defaults to 1 (previous row)."""
        # Partition by dept, order by salary ASC.
        # eng sorted: Bob 80000, Eve 85000, Alice 90000
        #   → lag(salary): NULL, 80000, 85000
        # sales sorted: Carol 70000, Dave 75000
        #   → lag(salary): NULL, 70000
        spec = PlanWindowFuncSpec(
            func="lag",
            arg_expr=_col("salary"),
            partition_by=(_col("dept"),),
            order_by=((_col("salary"), False),),
            alias="prev_sal",
            extra_args=(Literal(1), Literal(None)),
        )
        rows = _run([spec], ["dept", "name", "salary"], ["dept", "name", "salary", "prev_sal"])

        eng = {r[1]: r[3] for r in rows if r[0] == "eng"}
        assert eng["Bob"] is None       # first in partition
        assert eng["Eve"] == 80000      # previous was Bob
        assert eng["Alice"] == 85000    # previous was Eve

        sales = {r[1]: r[3] for r in rows if r[0] == "sales"}
        assert sales["Carol"] is None   # first in partition
        assert sales["Dave"] == 70000   # previous was Carol

    def test_lag_offset_2(self) -> None:
        """LAG with offset=2 skips one intermediate row."""
        # eng sorted by salary ASC: Bob(80k), Eve(85k), Alice(90k)
        # lag(salary, 2): NULL, NULL, 80000
        spec = PlanWindowFuncSpec(
            func="lag",
            arg_expr=_col("salary"),
            partition_by=(_col("dept"),),
            order_by=((_col("salary"), False),),
            alias="lag2",
            extra_args=(Literal(2), Literal(None)),
        )
        rows = _run([spec], ["dept", "name", "salary"], ["dept", "name", "salary", "lag2"])
        eng = sorted(
            [(r[2], r[3]) for r in rows if r[0] == "eng"],
            key=lambda x: x[0],
        )
        # eng by salary: 80000→None, 85000→None, 90000→80000
        assert eng[0] == (80000, None)
        assert eng[1] == (85000, None)
        assert eng[2] == (90000, 80000)

    def test_lag_with_default(self) -> None:
        """LAG with a non-NULL default fills boundary rows instead of NULL."""
        spec = PlanWindowFuncSpec(
            func="lag",
            arg_expr=_col("salary"),
            partition_by=(_col("dept"),),
            order_by=((_col("salary"), False),),
            alias="prev_sal",
            extra_args=(Literal(1), Literal(-1)),
        )
        rows = _run([spec], ["dept", "name", "salary"], ["dept", "name", "salary", "prev_sal"])
        eng = {r[1]: r[3] for r in rows if r[0] == "eng"}
        assert eng["Bob"] == -1         # first row uses the default
        assert eng["Eve"] == 80000

    def test_lag_global_no_partition(self) -> None:
        """LAG over all rows with no PARTITION BY — single global partition."""
        spec = PlanWindowFuncSpec(
            func="lag",
            arg_expr=_col("id"),
            partition_by=(),
            order_by=((_col("id"), False),),
            alias="prev_id",
            extra_args=(Literal(1), Literal(None)),
        )
        rows = _run([spec], ["id"], ["id", "prev_id"])
        # Ordered by id: 1→None, 2→1, 3→2, 4→3, 5→4
        by_id = {r[0]: r[1] for r in rows}
        assert by_id[1] is None
        assert by_id[2] == 1
        assert by_id[5] == 4


# ---------------------------------------------------------------------------
# LEAD
# ---------------------------------------------------------------------------


class TestLead:
    """LEAD(col [, offset=1 [, default=NULL]]) looks forward in an ordered partition."""

    def test_lead_default_offset(self) -> None:
        """LEAD with no explicit offset defaults to 1 (next row)."""
        # eng sorted by salary ASC: Bob 80000, Eve 85000, Alice 90000
        # lead(salary): 85000, 90000, NULL
        spec = PlanWindowFuncSpec(
            func="lead",
            arg_expr=_col("salary"),
            partition_by=(_col("dept"),),
            order_by=((_col("salary"), False),),
            alias="next_sal",
            extra_args=(Literal(1), Literal(None)),
        )
        rows = _run([spec], ["dept", "name", "salary"], ["dept", "name", "salary", "next_sal"])
        eng = {r[1]: r[3] for r in rows if r[0] == "eng"}
        assert eng["Bob"] == 85000      # next is Eve
        assert eng["Eve"] == 90000      # next is Alice
        assert eng["Alice"] is None     # last in partition

    def test_lead_offset_2(self) -> None:
        """LEAD with offset=2 skips one row ahead."""
        spec = PlanWindowFuncSpec(
            func="lead",
            arg_expr=_col("salary"),
            partition_by=(_col("dept"),),
            order_by=((_col("salary"), False),),
            alias="lead2",
            extra_args=(Literal(2), Literal(None)),
        )
        rows = _run([spec], ["dept", "name", "salary"], ["dept", "name", "salary", "lead2"])
        eng = sorted(
            [(r[2], r[3]) for r in rows if r[0] == "eng"],
            key=lambda x: x[0],
        )
        # eng by salary: 80000→90000, 85000→None, 90000→None
        assert eng[0] == (80000, 90000)
        assert eng[1] == (85000, None)
        assert eng[2] == (90000, None)

    def test_lead_with_default(self) -> None:
        """LEAD with a non-NULL default fills boundary rows."""
        spec = PlanWindowFuncSpec(
            func="lead",
            arg_expr=_col("salary"),
            partition_by=(_col("dept"),),
            order_by=((_col("salary"), False),),
            alias="next_sal",
            extra_args=(Literal(1), Literal(0)),
        )
        rows = _run([spec], ["dept", "name", "salary"], ["dept", "name", "salary", "next_sal"])
        eng = {r[1]: r[3] for r in rows if r[0] == "eng"}
        assert eng["Alice"] == 0        # last row uses the default


# ---------------------------------------------------------------------------
# NTILE
# ---------------------------------------------------------------------------


class TestNtile:
    """NTILE(n) distributes rows into n buckets numbered 1..n."""

    def test_ntile_3_buckets_global(self) -> None:
        """5 rows into 3 buckets: buckets 1,2 get 2 rows each; bucket 3 gets 1.

        Distribution: q, r = divmod(5, 3) → q=1, r=2
        Bucket 1: 2 rows, Bucket 2: 2 rows, Bucket 3: 1 row.
        """
        spec = PlanWindowFuncSpec(
            func="ntile",
            arg_expr=Literal(3),
            partition_by=(),
            order_by=((_col("id"), False),),
            alias="bucket",
            extra_args=(),
        )
        rows = _run([spec], ["id"], ["id", "bucket"])
        by_bucket: dict[int, list[int]] = {}
        for id_val, bucket in rows:
            by_bucket.setdefault(bucket, []).append(id_val)
        assert set(by_bucket.keys()) == {1, 2, 3}
        assert len(by_bucket[1]) == 2
        assert len(by_bucket[2]) == 2
        assert len(by_bucket[3]) == 1

    def test_ntile_2_buckets_partitioned(self) -> None:
        """NTILE(2) within each department.

        eng (3 rows): q, r = divmod(3, 2) → q=1, r=1
                      bucket 1: 2 rows, bucket 2: 1 row
        sales (2 rows): q, r = divmod(2, 2) → q=1, r=0
                        bucket 1: 1 row, bucket 2: 1 row
        """
        spec = PlanWindowFuncSpec(
            func="ntile",
            arg_expr=Literal(2),
            partition_by=(_col("dept"),),
            order_by=((_col("salary"), False),),
            alias="bucket",
            extra_args=(),
        )
        rows = _run([spec], ["dept", "salary"], ["dept", "salary", "bucket"])
        eng_rows = sorted([(r[1], r[2]) for r in rows if r[0] == "eng"])
        # eng sorted by salary ASC: 80000 → b1, 85000 → b1, 90000 → b2
        assert eng_rows[0][1] == 1
        assert eng_rows[1][1] == 1
        assert eng_rows[2][1] == 2

    def test_ntile_larger_than_partition(self) -> None:
        """NTILE(10) with only 2 rows: each row gets a distinct bucket 1..2.

        When n > partition_size, max(1, n) buckets but only partition_size
        of them are filled. Row i gets bucket i+1.
        """
        spec = PlanWindowFuncSpec(
            func="ntile",
            arg_expr=Literal(10),
            partition_by=(_col("dept"),),
            order_by=((_col("salary"), False),),
            alias="bucket",
            extra_args=(),
        )
        rows = _run([spec], ["dept", "salary"], ["dept", "salary", "bucket"])
        sales = sorted([(r[1], r[2]) for r in rows if r[0] == "sales"])
        # 2 rows into 10 buckets: Carol→1, Dave→2
        assert sales[0][1] == 1
        assert sales[1][1] == 2

    def test_ntile_1_bucket(self) -> None:
        """NTILE(1): every row goes into bucket 1."""
        spec = PlanWindowFuncSpec(
            func="ntile",
            arg_expr=Literal(1),
            partition_by=(),
            order_by=((_col("id"), False),),
            alias="bucket",
            extra_args=(),
        )
        rows = _run([spec], ["id"], ["id", "bucket"])
        assert all(r[1] == 1 for r in rows)


# ---------------------------------------------------------------------------
# PERCENT_RANK
# ---------------------------------------------------------------------------


class TestPercentRank:
    """PERCENT_RANK: (rank - 1) / (N - 1), range [0.0, 1.0]."""

    def test_percent_rank_global_unique_order(self) -> None:
        """5 distinct salaries → percent_rank values are 0, 0.25, 0.5, 0.75, 1.

        Salaries sorted ASC: 70000, 75000, 80000, 85000, 90000
        """
        spec = PlanWindowFuncSpec(
            func="percent_rank",
            arg_expr=None,
            partition_by=(),
            order_by=((_col("salary"), False),),
            alias="pr",
            extra_args=(),
        )
        rows = _run([spec], ["salary"], ["salary", "pr"])
        by_salary = {r[0]: r[1] for r in rows}
        assert abs(by_salary[70000] - 0.0) < 1e-9
        assert abs(by_salary[75000] - 0.25) < 1e-9
        assert abs(by_salary[80000] - 0.5) < 1e-9
        assert abs(by_salary[85000] - 0.75) < 1e-9
        assert abs(by_salary[90000] - 1.0) < 1e-9

    def test_percent_rank_with_ties(self) -> None:
        """Tied rows share the same PERCENT_RANK value.

        dept is the order key: 3 eng rows tie at rank 1, 2 sales rows tie at rank 4.
        N=5, so: eng → (1-1)/(5-1)=0.0; sales → (4-1)/(5-1)=0.75
        """
        spec = PlanWindowFuncSpec(
            func="percent_rank",
            arg_expr=None,
            partition_by=(),
            order_by=((_col("dept"), False),),  # "eng" < "sales" lexicographically
            alias="pr",
            extra_args=(),
        )
        rows = _run([spec], ["dept"], ["dept", "pr"])
        eng_vals = {r[1] for r in rows if r[0] == "eng"}
        sales_vals = {r[1] for r in rows if r[0] == "sales"}
        assert len(eng_vals) == 1
        assert abs(list(eng_vals)[0] - 0.0) < 1e-9
        assert len(sales_vals) == 1
        assert abs(list(sales_vals)[0] - 0.75) < 1e-9

    def test_percent_rank_single_row_partition(self) -> None:
        """A partition with a single row always returns 0.0 (N=1 special case)."""
        # Split into per-employee partitions (id is unique → each partition has 1 row).
        spec = PlanWindowFuncSpec(
            func="percent_rank",
            arg_expr=None,
            partition_by=(_col("id"),),
            order_by=((_col("salary"), False),),
            alias="pr",
            extra_args=(),
        )
        rows = _run([spec], ["id"], ["id", "pr"])
        assert all(r[1] == 0.0 for r in rows)


# ---------------------------------------------------------------------------
# CUME_DIST
# ---------------------------------------------------------------------------


class TestCumeDist:
    """CUME_DIST: (position of last peer row + 1) / N, range (0, 1]."""

    def test_cume_dist_global_unique(self) -> None:
        """5 distinct salaries → cume_dist = 0.2, 0.4, 0.6, 0.8, 1.0."""
        spec = PlanWindowFuncSpec(
            func="cume_dist",
            arg_expr=None,
            partition_by=(),
            order_by=((_col("salary"), False),),
            alias="cd",
            extra_args=(),
        )
        rows = _run([spec], ["salary"], ["salary", "cd"])
        by_salary = sorted(rows, key=lambda r: r[0])
        expected = [0.2, 0.4, 0.6, 0.8, 1.0]
        for (sal, cd), exp in zip(by_salary, expected, strict=True):
            assert abs(cd - exp) < 1e-9, f"salary={sal}: expected {exp}, got {cd}"

    def test_cume_dist_with_ties(self) -> None:
        """Tied rows share the peer-group end position.

        Order by dept ASC: 3 eng rows in peer group ending at pos 3 → cd=0.6;
                           2 sales rows ending at pos 5 → cd=1.0.
        """
        spec = PlanWindowFuncSpec(
            func="cume_dist",
            arg_expr=None,
            partition_by=(),
            order_by=((_col("dept"), False),),
            alias="cd",
            extra_args=(),
        )
        rows = _run([spec], ["dept"], ["dept", "cd"])
        eng_vals = {r[1] for r in rows if r[0] == "eng"}
        sales_vals = {r[1] for r in rows if r[0] == "sales"}
        assert len(eng_vals) == 1 and abs(list(eng_vals)[0] - 0.6) < 1e-9
        assert len(sales_vals) == 1 and abs(list(sales_vals)[0] - 1.0) < 1e-9

    def test_cume_dist_partitioned(self) -> None:
        """CUME_DIST within each department partition.

        eng (3 rows, order by salary ASC): 80000→1/3, 85000→2/3, 90000→1.0
        sales (2 rows): 70000→0.5, 75000→1.0
        """
        spec = PlanWindowFuncSpec(
            func="cume_dist",
            arg_expr=None,
            partition_by=(_col("dept"),),
            order_by=((_col("salary"), False),),
            alias="cd",
            extra_args=(),
        )
        rows = _run([spec], ["dept", "salary"], ["dept", "salary", "cd"])
        eng = sorted([(r[1], r[2]) for r in rows if r[0] == "eng"])
        assert abs(eng[0][1] - 1 / 3) < 1e-9
        assert abs(eng[1][1] - 2 / 3) < 1e-9
        assert abs(eng[2][1] - 1.0) < 1e-9

        sales = sorted([(r[1], r[2]) for r in rows if r[0] == "sales"])
        assert abs(sales[0][1] - 0.5) < 1e-9
        assert abs(sales[1][1] - 1.0) < 1e-9


# ---------------------------------------------------------------------------
# NTH_VALUE
# ---------------------------------------------------------------------------


class TestNthValue:
    """NTH_VALUE(col, n): value of col at the n-th row (1-indexed) of the partition."""

    def test_nth_value_first_row(self) -> None:
        """NTH_VALUE(salary, 1) == FIRST_VALUE(salary)."""
        spec = PlanWindowFuncSpec(
            func="nth_value",
            arg_expr=_col("salary"),
            partition_by=(_col("dept"),),
            order_by=((_col("salary"), False),),
            alias="nth",
            extra_args=(Literal(1),),
        )
        rows = _run([spec], ["dept", "salary"], ["dept", "salary", "nth"])
        eng = {r[2] for r in rows if r[0] == "eng"}
        # Lowest salary in eng is 80000 (Bob), sorted ASC
        assert eng == {80000}

    def test_nth_value_second_row(self) -> None:
        """NTH_VALUE(salary, 2) returns the 2nd-smallest salary in each partition."""
        spec = PlanWindowFuncSpec(
            func="nth_value",
            arg_expr=_col("salary"),
            partition_by=(_col("dept"),),
            order_by=((_col("salary"), False),),
            alias="nth",
            extra_args=(Literal(2),),
        )
        rows = _run([spec], ["dept", "salary"], ["dept", "salary", "nth"])
        eng = {r[2] for r in rows if r[0] == "eng"}
        # eng sorted ASC: Bob 80000, Eve 85000, Alice 90000  → 2nd = 85000
        assert eng == {85000}

        sales = {r[2] for r in rows if r[0] == "sales"}
        # sales sorted ASC: Carol 70000, Dave 75000  → 2nd = 75000
        assert sales == {75000}

    def test_nth_value_beyond_partition_size(self) -> None:
        """NTH_VALUE with n > partition size returns NULL for all rows."""
        spec = PlanWindowFuncSpec(
            func="nth_value",
            arg_expr=_col("salary"),
            partition_by=(_col("dept"),),
            order_by=((_col("salary"), False),),
            alias="nth",
            extra_args=(Literal(99),),
        )
        rows = _run([spec], ["dept", "salary"], ["dept", "salary", "nth"])
        assert all(r[2] is None for r in rows)

    def test_nth_value_global_no_partition(self) -> None:
        """NTH_VALUE(salary, 3) over all 5 rows (no partition) → the 3rd smallest."""
        spec = PlanWindowFuncSpec(
            func="nth_value",
            arg_expr=_col("salary"),
            partition_by=(),
            order_by=((_col("salary"), False),),
            alias="nth",
            extra_args=(Literal(3),),
        )
        rows = _run([spec], ["salary"], ["salary", "nth"])
        # All 5 salaries ASC: 70000, 75000, 80000, 85000, 90000  → 3rd = 80000
        assert all(r[1] == 80000 for r in rows)


# ---------------------------------------------------------------------------
# Edge cases and combinations
# ---------------------------------------------------------------------------


class TestEdgeCases:
    def test_lag_single_row_partition(self) -> None:
        """A single-row partition always has LAG = default (NULL if unspecified)."""
        # Partition by id (each id is unique → 1 row per partition).
        spec = PlanWindowFuncSpec(
            func="lag",
            arg_expr=_col("salary"),
            partition_by=(_col("id"),),
            order_by=((_col("salary"), False),),
            alias="prev_sal",
            extra_args=(Literal(1), Literal(None)),
        )
        rows = _run([spec], ["id", "salary"], ["id", "salary", "prev_sal"])
        # Every partition has 1 row → no predecessor → all NULL.
        assert all(r[2] is None for r in rows)

    def test_lead_single_row_partition(self) -> None:
        """A single-row partition always has LEAD = default (NULL if unspecified)."""
        spec = PlanWindowFuncSpec(
            func="lead",
            arg_expr=_col("salary"),
            partition_by=(_col("id"),),
            order_by=((_col("salary"), False),),
            alias="next_sal",
            extra_args=(Literal(1), Literal(None)),
        )
        rows = _run([spec], ["id", "salary"], ["id", "salary", "next_sal"])
        assert all(r[2] is None for r in rows)

    def test_lag_and_lead_combined(self) -> None:
        """LAG and LEAD can run in the same WindowAgg node without interfering."""
        spec_lag = PlanWindowFuncSpec(
            func="lag",
            arg_expr=_col("salary"),
            partition_by=(_col("dept"),),
            order_by=((_col("salary"), False),),
            alias="prev_sal",
            extra_args=(Literal(1), Literal(None)),
        )
        spec_lead = PlanWindowFuncSpec(
            func="lead",
            arg_expr=_col("salary"),
            partition_by=(_col("dept"),),
            order_by=((_col("salary"), False),),
            alias="next_sal",
            extra_args=(Literal(1), Literal(None)),
        )
        rows = _run(
            [spec_lag, spec_lead],
            ["dept", "salary"],
            ["dept", "salary", "prev_sal", "next_sal"],
        )
        # eng sorted ASC: Bob 80000, Eve 85000, Alice 90000
        eng = sorted([(r[1], r[2], r[3]) for r in rows if r[0] == "eng"])
        assert eng[0] == (80000, None, 85000)   # Bob: no prev, next=Eve
        assert eng[1] == (85000, 80000, 90000)  # Eve: prev=Bob, next=Alice
        assert eng[2] == (90000, 85000, None)   # Alice: prev=Eve, no next

    def test_percent_rank_and_cume_dist_together(self) -> None:
        """PERCENT_RANK and CUME_DIST produce complementary values in the same node."""
        spec_pr = PlanWindowFuncSpec(
            func="percent_rank",
            arg_expr=None,
            partition_by=(_col("dept"),),
            order_by=((_col("salary"), False),),
            alias="pr",
            extra_args=(),
        )
        spec_cd = PlanWindowFuncSpec(
            func="cume_dist",
            arg_expr=None,
            partition_by=(_col("dept"),),
            order_by=((_col("salary"), False),),
            alias="cd",
            extra_args=(),
        )
        rows = _run(
            [spec_pr, spec_cd],
            ["dept", "salary"],
            ["dept", "salary", "pr", "cd"],
        )
        # eng sorted ASC: Bob 80000, Eve 85000, Alice 90000 (N=3)
        # percent_rank: 0, 0.5, 1.0
        # cume_dist:    1/3, 2/3, 1.0
        eng = sorted([(r[1], r[2], r[3]) for r in rows if r[0] == "eng"])
        assert abs(eng[0][1] - 0.0) < 1e-9
        assert abs(eng[1][1] - 0.5) < 1e-9
        assert abs(eng[2][1] - 1.0) < 1e-9
        assert abs(eng[0][2] - 1 / 3) < 1e-9
        assert abs(eng[1][2] - 2 / 3) < 1e-9
        assert abs(eng[2][2] - 1.0) < 1e-9

    def test_ntile_no_extra_args_in_ir_defaults_to_1(self) -> None:
        """NTILE WinFuncSpec with empty extra_args: VM uses n=max(1,1)=1 fallback.

        This exercises the defensive ``(n_buckets_raw,) = spec.extra_args if
        spec.extra_args else (1,)`` branch in the VM's NTILE handler.  That
        branch is only reachable by constructing the IR directly (the codegen
        always populates extra_args for NTILE), so we drive the VM's
        _do_compute_window helper directly on a hand-built _MutableResult.
        """
        import types

        from sql_codegen import ComputeWindowFunctions
        from sql_codegen.ir import WinFunc, WinFuncSpec

        from sql_vm.result import _MutableResult
        from sql_vm.vm import _do_compute_window

        spec_ir = WinFuncSpec(
            func=WinFunc.NTILE,
            arg_col=None,
            partition_cols=(),
            order_cols=(("id", False),),
            result_col="bucket",
            extra_args=(),    # empty → VM should default to n=1
        )
        instr = ComputeWindowFunctions(
            specs=(spec_ir,),
            output_cols=("id", "bucket"),
        )
        # _do_compute_window only reads st.result.columns and st.result.rows,
        # so a SimpleNamespace with those two attributes is sufficient.
        result_buf = _MutableResult(
            columns=("id",),
            rows=[(1,), (2,), (3,)],
        )
        state = types.SimpleNamespace(result=result_buf)
        _do_compute_window(instr, state)  # type: ignore[arg-type]
        # With n=1 (default from empty extra_args), every row lands in bucket 1.
        assert all(r[1] == 1 for r in state.result.rows)

    def test_lag_vm_guard_rejects_string_offset(self) -> None:
        """Defense-in-depth: LAG handler raises RuntimeError for non-int offset.

        The codegen already rejects this at compile time; this test reaches the
        VM-level guard via a hand-crafted WinFuncSpec to ensure the guard is
        exercised and does not emit a leaky ValueError.
        """
        import types

        import pytest
        from sql_codegen import ComputeWindowFunctions
        from sql_codegen.ir import WinFunc, WinFuncSpec

        from sql_vm.result import _MutableResult
        from sql_vm.vm import _do_compute_window

        spec_ir = WinFuncSpec(
            func=WinFunc.LAG,
            arg_col="salary",
            partition_cols=(),
            order_cols=(("id", False),),
            result_col="prev_sal",
            extra_args=("two", None),   # string offset — invalid
        )
        instr = ComputeWindowFunctions(
            specs=(spec_ir,),
            output_cols=("id", "salary", "prev_sal"),
        )
        result_buf = _MutableResult(
            columns=("id", "salary"),
            rows=[(1, 100), (2, 200)],
        )
        state = types.SimpleNamespace(result=result_buf)
        with pytest.raises(RuntimeError, match="LAG offset must be an integer"):
            _do_compute_window(instr, state)  # type: ignore[arg-type]

    def test_ntile_vm_guard_rejects_string_n(self) -> None:
        """Defense-in-depth: NTILE handler raises RuntimeError for non-int n.

        As with the LAG guard above, codegen prevents this in practice; we
        drive the VM directly to confirm the defensive check is reachable.
        """
        import types

        import pytest
        from sql_codegen import ComputeWindowFunctions
        from sql_codegen.ir import WinFunc, WinFuncSpec

        from sql_vm.result import _MutableResult
        from sql_vm.vm import _do_compute_window

        spec_ir = WinFuncSpec(
            func=WinFunc.NTILE,
            arg_col=None,
            partition_cols=(),
            order_cols=(("id", False),),
            result_col="bucket",
            extra_args=("three",),  # string bucket count — invalid
        )
        instr = ComputeWindowFunctions(
            specs=(spec_ir,),
            output_cols=("id", "bucket"),
        )
        result_buf = _MutableResult(
            columns=("id",),
            rows=[(1,), (2,), (3,)],
        )
        state = types.SimpleNamespace(result=result_buf)
        with pytest.raises(RuntimeError, match="NTILE n must be an integer"):
            _do_compute_window(instr, state)  # type: ignore[arg-type]

    def test_nth_value_vm_guard_rejects_string_n(self) -> None:
        """Defense-in-depth: NTH_VALUE handler raises RuntimeError for non-int n."""
        import types

        import pytest
        from sql_codegen import ComputeWindowFunctions
        from sql_codegen.ir import WinFunc, WinFuncSpec

        from sql_vm.result import _MutableResult
        from sql_vm.vm import _do_compute_window

        spec_ir = WinFuncSpec(
            func=WinFunc.NTH_VALUE,
            arg_col="salary",
            partition_cols=(),
            order_cols=(("id", False),),
            result_col="nv",
            extra_args=("two",),  # string row index — invalid
        )
        instr = ComputeWindowFunctions(
            specs=(spec_ir,),
            output_cols=("id", "salary", "nv"),
        )
        result_buf = _MutableResult(
            columns=("id", "salary"),
            rows=[(1, 100), (2, 200)],
        )
        state = types.SimpleNamespace(result=result_buf)
        with pytest.raises(RuntimeError, match="NTH_VALUE n must be an integer"):
            _do_compute_window(instr, state)  # type: ignore[arg-type]
