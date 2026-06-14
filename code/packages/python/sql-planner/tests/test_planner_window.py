"""Window function planning — WindowAgg plan node construction."""

from __future__ import annotations

from sql_planner import (
    Column,
    InMemorySchemaProvider,
    Project,
    SelectItem,
    SelectStmt,
    Sort,
    TableRef,
    WindowAgg,
    WindowFuncExpr,
    plan,
)
from sql_planner.ast import Limit as AstLimit
from sql_planner.ast import SortKey as AstSortKey
from sql_planner.plan import Limit as PlanLimit


def schema() -> InMemorySchemaProvider:
    return InMemorySchemaProvider({"employees": ["id", "name", "dept", "salary"]})


def _plan_window(
    sql_items: tuple,
    order_by: tuple = (),
    limit: AstLimit | None = None,
) -> object:
    """Build a SelectStmt with the given items and run plan()."""
    ast = SelectStmt(
        from_=TableRef(table="employees"),
        items=sql_items,
        order_by=order_by,
        limit=limit,
    )
    return plan(ast, schema())


def _row_number_item(alias: str = "rn") -> SelectItem:
    return SelectItem(expr=WindowFuncExpr(func="row_number", arg=None), alias=alias)


def _sum_salary_item(alias: str = "total") -> SelectItem:
    return SelectItem(
        expr=WindowFuncExpr(
            func="sum",
            arg=Column(table=None, col="salary"),
            partition_by=(Column(table=None, col="dept"),),
        ),
        alias=alias,
    )


# ---------------------------------------------------------------------------
# Basic WindowAgg node structure
# ---------------------------------------------------------------------------


class TestWindowAggNode:
    def test_row_number_produces_window_agg(self) -> None:
        p = _plan_window((_row_number_item(),))
        assert isinstance(p, WindowAgg)

    def test_window_agg_output_cols_includes_alias(self) -> None:
        p = _plan_window((_row_number_item("row_num"),))
        assert isinstance(p, WindowAgg)
        assert "row_num" in p.output_cols

    def test_non_window_cols_in_output(self) -> None:
        items = (
            SelectItem(expr=Column(table=None, col="name"), alias=None),
            _row_number_item("rn"),
        )
        p = _plan_window(items)
        assert isinstance(p, WindowAgg)
        assert "name" in p.output_cols
        assert "rn" in p.output_cols

    def test_inner_plan_is_project(self) -> None:
        p = _plan_window((_row_number_item(),))
        assert isinstance(p, WindowAgg)
        assert isinstance(p.input, Project)

    def test_specs_tuple_length(self) -> None:
        items = (
            _row_number_item("rn"),
            _sum_salary_item("dept_total"),
        )
        p = _plan_window(items)
        assert isinstance(p, WindowAgg)
        assert len(p.specs) == 2

    def test_spec_func_name(self) -> None:
        p = _plan_window((_row_number_item(),))
        assert isinstance(p, WindowAgg)
        assert p.specs[0].func == "row_number"

    def test_spec_alias(self) -> None:
        p = _plan_window((_row_number_item("my_rank"),))
        assert isinstance(p, WindowAgg)
        assert p.specs[0].alias == "my_rank"


# ---------------------------------------------------------------------------
# Partition BY and ORDER BY in specs
# ---------------------------------------------------------------------------


class TestWindowSpec:
    def test_partition_by_resolves_column(self) -> None:
        item = SelectItem(
            expr=WindowFuncExpr(
                func="sum",
                arg=Column(table=None, col="salary"),
                partition_by=(Column(table=None, col="dept"),),
            ),
            alias="dept_total",
        )
        p = _plan_window((item,))
        assert isinstance(p, WindowAgg)
        spec = p.specs[0]
        assert len(spec.partition_by) == 1
        pb_col = spec.partition_by[0]
        assert isinstance(pb_col, Column)
        assert pb_col.col == "dept"

    def test_order_by_in_spec(self) -> None:
        item = SelectItem(
            expr=WindowFuncExpr(
                func="rank",
                arg=None,
                order_by=((Column(table=None, col="salary"), True),),
            ),
            alias="r",
        )
        p = _plan_window((item,))
        assert isinstance(p, WindowAgg)
        spec = p.specs[0]
        assert len(spec.order_by) == 1
        ob_col, descending = spec.order_by[0]
        assert isinstance(ob_col, Column)
        assert ob_col.col == "salary"
        assert descending is True

    def test_arg_expr_in_spec(self) -> None:
        item = SelectItem(
            expr=WindowFuncExpr(
                func="sum",
                arg=Column(table=None, col="salary"),
            ),
            alias="s",
        )
        p = _plan_window((item,))
        assert isinstance(p, WindowAgg)
        spec = p.specs[0]
        assert spec.arg_expr is not None
        assert isinstance(spec.arg_expr, Column)
        assert spec.arg_expr.col == "salary"


# ---------------------------------------------------------------------------
# Dependency column injection
# ---------------------------------------------------------------------------


class TestDependencyColumns:
    def test_partition_col_added_to_inner_project(self) -> None:
        """partition_by col not in non-window items must be injected into inner Project."""
        item = SelectItem(
            expr=WindowFuncExpr(
                func="row_number",
                arg=None,
                partition_by=(Column(table=None, col="dept"),),
            ),
            alias="rn",
        )
        p = _plan_window((item,))
        assert isinstance(p, WindowAgg)
        inner = p.input
        assert isinstance(inner, Project)
        inner_col_names = [
            pi.alias or (pi.expr.col if isinstance(pi.expr, Column) else None)
            for pi in inner.items
        ]
        assert "dept" in inner_col_names

    def test_non_window_cols_not_duplicated(self) -> None:
        """dep col already in non-window SELECT items is not added twice."""
        items = (
            SelectItem(expr=Column(table=None, col="dept"), alias=None),
            SelectItem(
                expr=WindowFuncExpr(
                    func="row_number",
                    arg=None,
                    partition_by=(Column(table=None, col="dept"),),
                ),
                alias="rn",
            ),
        )
        p = _plan_window(items)
        assert isinstance(p, WindowAgg)
        inner = p.input
        assert isinstance(inner, Project)
        dept_count = sum(
            1 for pi in inner.items
            if isinstance(pi.expr, Column) and pi.expr.col == "dept"
        )
        assert dept_count == 1


# ---------------------------------------------------------------------------
# WindowAgg wrapped in Sort / Limit
# ---------------------------------------------------------------------------


class TestWindowAggWithWrappers:
    def test_order_by_wraps_window_agg_in_sort(self) -> None:
        items = (
            SelectItem(expr=Column(table=None, col="name"), alias=None),
            _row_number_item("rn"),
        )
        p = _plan_window(
            items,
            order_by=(AstSortKey(expr=Column(table=None, col="name"), descending=False),),
        )
        assert isinstance(p, Sort)
        assert isinstance(p.input, WindowAgg)

    def test_limit_wraps_window_agg(self) -> None:
        p = _plan_window(
            (_row_number_item("rn"),),
            limit=AstLimit(count=5, offset=None),
        )
        assert isinstance(p, PlanLimit)
        assert isinstance(p.input, WindowAgg)

    def test_order_by_and_limit_wrap_window_agg(self) -> None:
        items = (
            SelectItem(expr=Column(table=None, col="name"), alias=None),
            _row_number_item("rn"),
        )
        p = _plan_window(
            items,
            order_by=(AstSortKey(expr=Column(table=None, col="name"), descending=False),),
            limit=AstLimit(count=3, offset=None),
        )
        assert isinstance(p, PlanLimit)
        assert isinstance(p.input, Sort)
        assert isinstance(p.input.input, WindowAgg)


# ---------------------------------------------------------------------------
# Multiple window functions
# ---------------------------------------------------------------------------


class TestMultipleWindowFuncs:
    def test_two_window_funcs_produce_two_specs(self) -> None:
        items = (
            _row_number_item("rn"),
            SelectItem(
                expr=WindowFuncExpr(func="dense_rank", arg=None),
                alias="dr",
            ),
        )
        p = _plan_window(items)
        assert isinstance(p, WindowAgg)
        assert len(p.specs) == 2
        funcs = {s.func for s in p.specs}
        assert "row_number" in funcs
        assert "dense_rank" in funcs

    def test_output_cols_order(self) -> None:
        """Non-window cols come before window alias cols in output_cols."""
        items = (
            SelectItem(expr=Column(table=None, col="name"), alias=None),
            SelectItem(expr=Column(table=None, col="dept"), alias=None),
            _row_number_item("rn"),
        )
        p = _plan_window(items)
        assert isinstance(p, WindowAgg)
        name_idx = p.output_cols.index("name")
        dept_idx = p.output_cols.index("dept")
        rn_idx = p.output_cols.index("rn")
        assert name_idx < rn_idx
        assert dept_idx < rn_idx


# ---------------------------------------------------------------------------
# _resolve for WindowFuncExpr — column qualification
# ---------------------------------------------------------------------------


class TestWindowFuncExprResolve:
    def test_bare_column_in_arg_resolves(self) -> None:
        """Column in arg_expr is resolved to a fully qualified Column."""
        item = SelectItem(
            expr=WindowFuncExpr(
                func="sum",
                arg=Column(table=None, col="salary"),
            ),
            alias="s",
        )
        p = _plan_window((item,))
        assert isinstance(p, WindowAgg)
        spec = p.specs[0]
        assert spec.arg_expr is not None
        assert isinstance(spec.arg_expr, Column)
        assert spec.arg_expr.table == "employees"

    def test_arg_none_stays_none(self) -> None:
        """arg=None (for ROW_NUMBER) stays None after resolve."""
        p = _plan_window((_row_number_item(),))
        assert isinstance(p, WindowAgg)
        assert p.specs[0].arg_expr is None
