"""
Window function codegen tests — _to_ir_win_spec and ComputeWindowFunctions emission.

These tests verify that ``compile(WindowAgg(...))`` emits a
``ComputeWindowFunctions`` instruction whose ``WinFuncSpec`` objects have the
correct ``func``, ``arg_col``, ``partition_cols``, ``order_cols``,
``result_col``, and ``extra_args`` values.

Coverage targets
----------------
- compiler.py: _to_ir_win_spec — all branches for LAG, LEAD, NTILE,
  NTH_VALUE, PERCENT_RANK, CUME_DIST, plus error paths
- ir.py: WinFunc enum values for the new functions
"""

from __future__ import annotations

import pytest
from sql_planner import (
    Column,
    Literal,
    Project,
    ProjectionItem,
    Scan,
    WindowAgg,
)
from sql_planner.plan import WindowFuncSpec as PlanWindowFuncSpec

from sql_codegen import ComputeWindowFunctions, WinFunc, compile
from sql_codegen.errors import UnsupportedNode

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _compile_window(
    specs: list[PlanWindowFuncSpec], output_cols: list[str]
) -> ComputeWindowFunctions:
    """Compile a WindowAgg plan and return its ComputeWindowFunctions instruction."""
    col_items = tuple(
        ProjectionItem(expr=Column("t", c), alias=None)
        for c in ["a", "b", "c"]
    )
    inner = Project(input=Scan(table="t", alias="t"), items=col_items)
    plan = WindowAgg(
        input=inner,
        specs=tuple(specs),
        output_cols=tuple(output_cols),
    )
    prog = compile(plan)
    cwf = [i for i in prog.instructions if isinstance(i, ComputeWindowFunctions)]
    assert len(cwf) == 1, f"Expected exactly 1 ComputeWindowFunctions, got {len(cwf)}"
    return cwf[0]


def _col(name: str) -> Column:
    return Column(table="t", col=name)


# ---------------------------------------------------------------------------
# LAG
# ---------------------------------------------------------------------------


class TestLagCodegen:
    def test_lag_no_extra_args_normalises_to_offset1_defaultnull(self) -> None:
        """LAG(col) with no explicit offset/default → extra_args=(1, None)."""
        spec = PlanWindowFuncSpec(
            func="lag",
            arg_expr=_col("a"),
            partition_by=(_col("b"),),
            order_by=((_col("c"), False),),
            alias="prev_a",
            extra_args=(),
        )
        cwf = _compile_window([spec], ["a", "b", "c", "prev_a"])
        assert len(cwf.specs) == 1
        ws = cwf.specs[0]
        assert ws.func == WinFunc.LAG
        assert ws.arg_col == "a"
        assert ws.partition_cols == ("b",)
        assert ws.order_cols == (("c", False),)
        assert ws.result_col == "prev_a"
        assert ws.extra_args == (1, None)

    def test_lag_with_offset(self) -> None:
        """LAG(col, 3) → extra_args=(3, None)."""
        spec = PlanWindowFuncSpec(
            func="lag",
            arg_expr=_col("a"),
            partition_by=(),
            order_by=((_col("c"), False),),
            alias="lag3",
            extra_args=(Literal(3),),
        )
        cwf = _compile_window([spec], ["a", "c", "lag3"])
        ws = cwf.specs[0]
        assert ws.func == WinFunc.LAG
        assert ws.extra_args == (3, None)

    def test_lag_with_offset_and_default(self) -> None:
        """LAG(col, 2, -1) → extra_args=(2, -1)."""
        spec = PlanWindowFuncSpec(
            func="lag",
            arg_expr=_col("a"),
            partition_by=(),
            order_by=((_col("c"), False),),
            alias="lag2",
            extra_args=(Literal(2), Literal(-1)),
        )
        cwf = _compile_window([spec], ["a", "c", "lag2"])
        ws = cwf.specs[0]
        assert ws.extra_args == (2, -1)

    def test_lag_desc_order(self) -> None:
        """LAG with DESC order_by sets order_col desc flag correctly."""
        spec = PlanWindowFuncSpec(
            func="lag",
            arg_expr=_col("a"),
            partition_by=(),
            order_by=((_col("c"), True),),  # DESC
            alias="l",
            extra_args=(Literal(1), Literal(None)),
        )
        cwf = _compile_window([spec], ["a", "c", "l"])
        ws = cwf.specs[0]
        assert ws.order_cols == (("c", True),)


# ---------------------------------------------------------------------------
# LEAD
# ---------------------------------------------------------------------------


class TestLeadCodegen:
    def test_lead_no_extra_args_normalises(self) -> None:
        """LEAD(col) with no offset/default → extra_args=(1, None)."""
        spec = PlanWindowFuncSpec(
            func="lead",
            arg_expr=_col("a"),
            partition_by=(),
            order_by=((_col("c"), False),),
            alias="next_a",
            extra_args=(),
        )
        cwf = _compile_window([spec], ["a", "c", "next_a"])
        ws = cwf.specs[0]
        assert ws.func == WinFunc.LEAD
        assert ws.extra_args == (1, None)

    def test_lead_with_offset_and_default(self) -> None:
        """LEAD(col, 2, 0) → extra_args=(2, 0)."""
        spec = PlanWindowFuncSpec(
            func="lead",
            arg_expr=_col("a"),
            partition_by=(),
            order_by=((_col("c"), False),),
            alias="l",
            extra_args=(Literal(2), Literal(0)),
        )
        cwf = _compile_window([spec], ["a", "c", "l"])
        ws = cwf.specs[0]
        assert ws.extra_args == (2, 0)


# ---------------------------------------------------------------------------
# NTILE
# ---------------------------------------------------------------------------


class TestNtileCodegen:
    def test_ntile_moves_literal_arg_to_extra_args(self) -> None:
        """NTILE(4): bucket count moves from arg_expr to extra_args=(4,), arg_col=None."""
        spec = PlanWindowFuncSpec(
            func="ntile",
            arg_expr=Literal(4),
            partition_by=(_col("b"),),
            order_by=((_col("c"), False),),
            alias="bucket",
            extra_args=(),
        )
        cwf = _compile_window([spec], ["b", "c", "bucket"])
        ws = cwf.specs[0]
        assert ws.func == WinFunc.NTILE
        assert ws.arg_col is None          # literal bucket count, not a column
        assert ws.extra_args == (4,)
        assert ws.partition_cols == ("b",)

    def test_ntile_missing_arg_raises(self) -> None:
        """NTILE with arg_expr=None raises UnsupportedNode."""
        spec = PlanWindowFuncSpec(
            func="ntile",
            arg_expr=None,
            partition_by=(),
            order_by=(),
            alias="bucket",
            extra_args=(),
        )
        with pytest.raises(UnsupportedNode, match="NTILE requires a bucket-count argument"):
            _compile_window([spec], ["bucket"])

    def test_ntile_non_literal_arg_raises(self) -> None:
        """NTILE(col) — a column reference as bucket count — raises UnsupportedNode."""
        spec = PlanWindowFuncSpec(
            func="ntile",
            arg_expr=_col("a"),        # column, not a literal
            partition_by=(),
            order_by=(),
            alias="bucket",
            extra_args=(),
        )
        with pytest.raises(UnsupportedNode):
            _compile_window([spec], ["a", "bucket"])


# ---------------------------------------------------------------------------
# NTH_VALUE
# ---------------------------------------------------------------------------


class TestNthValueCodegen:
    def test_nth_value_col_and_n(self) -> None:
        """NTH_VALUE(col, 3): col → arg_col, 3 → extra_args=(3,)."""
        spec = PlanWindowFuncSpec(
            func="nth_value",
            arg_expr=_col("a"),
            partition_by=(_col("b"),),
            order_by=((_col("c"), False),),
            alias="nth",
            extra_args=(Literal(3),),
        )
        cwf = _compile_window([spec], ["a", "b", "c", "nth"])
        ws = cwf.specs[0]
        assert ws.func == WinFunc.NTH_VALUE
        assert ws.arg_col == "a"
        assert ws.extra_args == (3,)

    def test_nth_value_missing_n_raises(self) -> None:
        """NTH_VALUE with no extra_args raises UnsupportedNode."""
        spec = PlanWindowFuncSpec(
            func="nth_value",
            arg_expr=_col("a"),
            partition_by=(),
            order_by=(),
            alias="nth",
            extra_args=(),            # missing n
        )
        with pytest.raises(UnsupportedNode, match="NTH_VALUE requires two arguments"):
            _compile_window([spec], ["a", "nth"])

    def test_nth_value_non_literal_n_raises(self) -> None:
        """NTH_VALUE(col, other_col) raises because n must be a literal."""
        spec = PlanWindowFuncSpec(
            func="nth_value",
            arg_expr=_col("a"),
            partition_by=(),
            order_by=(),
            alias="nth",
            extra_args=(_col("b"),),   # column ref instead of literal
        )
        with pytest.raises(UnsupportedNode):
            _compile_window([spec], ["a", "b", "nth"])


# ---------------------------------------------------------------------------
# PERCENT_RANK and CUME_DIST
# ---------------------------------------------------------------------------


class TestPercentRankCumeDistCodegen:
    def test_percent_rank_no_extra_args(self) -> None:
        """PERCENT_RANK: arg_col=None, extra_args=() always."""
        spec = PlanWindowFuncSpec(
            func="percent_rank",
            arg_expr=None,
            partition_by=(_col("b"),),
            order_by=((_col("c"), False),),
            alias="pr",
            extra_args=(),
        )
        cwf = _compile_window([spec], ["b", "c", "pr"])
        ws = cwf.specs[0]
        assert ws.func == WinFunc.PERCENT_RANK
        assert ws.arg_col is None
        assert ws.extra_args == ()

    def test_cume_dist_no_extra_args(self) -> None:
        """CUME_DIST: arg_col=None, extra_args=() always."""
        spec = PlanWindowFuncSpec(
            func="cume_dist",
            arg_expr=None,
            partition_by=(),
            order_by=((_col("c"), True),),
            alias="cd",
            extra_args=(),
        )
        cwf = _compile_window([spec], ["c", "cd"])
        ws = cwf.specs[0]
        assert ws.func == WinFunc.CUME_DIST
        assert ws.arg_col is None
        assert ws.extra_args == ()
        assert ws.order_cols == (("c", True),)


# ---------------------------------------------------------------------------
# Multiple specs in one WindowAgg
# ---------------------------------------------------------------------------


class TestMultipleWindowSpecCodegen:
    def test_lag_and_lead_in_same_node(self) -> None:
        """Two window specs with different functions in a single ComputeWindowFunctions."""
        spec_lag = PlanWindowFuncSpec(
            func="lag",
            arg_expr=_col("a"),
            partition_by=(),
            order_by=((_col("c"), False),),
            alias="prev_a",
            extra_args=(),
        )
        spec_lead = PlanWindowFuncSpec(
            func="lead",
            arg_expr=_col("a"),
            partition_by=(),
            order_by=((_col("c"), False),),
            alias="next_a",
            extra_args=(),
        )
        cwf = _compile_window([spec_lag, spec_lead], ["a", "c", "prev_a", "next_a"])
        assert len(cwf.specs) == 2
        funcs = {ws.func for ws in cwf.specs}
        assert funcs == {WinFunc.LAG, WinFunc.LEAD}

    def test_ntile_and_percent_rank_together(self) -> None:
        """NTILE and PERCENT_RANK can coexist in the same WindowAgg."""
        spec_nt = PlanWindowFuncSpec(
            func="ntile",
            arg_expr=Literal(5),
            partition_by=(),
            order_by=((_col("c"), False),),
            alias="bucket",
            extra_args=(),
        )
        spec_pr = PlanWindowFuncSpec(
            func="percent_rank",
            arg_expr=None,
            partition_by=(),
            order_by=((_col("c"), False),),
            alias="pr",
            extra_args=(),
        )
        cwf = _compile_window([spec_nt, spec_pr], ["c", "bucket", "pr"])
        assert len(cwf.specs) == 2
        nt_spec = next(ws for ws in cwf.specs if ws.func == WinFunc.NTILE)
        assert nt_spec.extra_args == (5,)


# ---------------------------------------------------------------------------
# Error paths
# ---------------------------------------------------------------------------


class TestWindowCodegenErrors:
    def test_unknown_window_function_raises(self) -> None:
        """Compiling an unknown window function name raises UnsupportedNode."""
        spec = PlanWindowFuncSpec(
            func="funky_unknown_fn",
            arg_expr=None,
            partition_by=(),
            order_by=(),
            alias="x",
            extra_args=(),
        )
        with pytest.raises(UnsupportedNode, match="unknown window function"):
            _compile_window([spec], ["x"])

    def test_lag_extra_arg_non_literal_raises(self) -> None:
        """LAG extra arg that is not a Literal raises UnsupportedNode."""
        spec = PlanWindowFuncSpec(
            func="lag",
            arg_expr=_col("a"),
            partition_by=(),
            order_by=(),
            alias="l",
            extra_args=(_col("b"),),   # column ref, not literal
        )
        with pytest.raises(UnsupportedNode, match="LAG/LEAD offset"):
            _compile_window([spec], ["a", "b", "l"])
