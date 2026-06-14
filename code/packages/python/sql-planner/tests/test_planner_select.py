"""SELECT planning — WHERE, ORDER BY, LIMIT, DISTINCT, projection, alias derivation."""

from __future__ import annotations

from sql_planner import (
    BinaryExpr,
    BinaryOp,
    Column,
    Distinct,
    ExistsSubquery,
    Filter,
    FuncArg,
    FunctionCall,
    InMemorySchemaProvider,
    Literal,
    Project,
    ProjectionItem,
    Scan,
    SelectItem,
    SelectStmt,
    Sort,
    TableRef,
    Wildcard,
    plan,
    plan_all,
)
from sql_planner import (
    Limit as AstLimit,
)
from sql_planner import (
    SortKey as AstSortKey,
)
from sql_planner.plan import Limit as PlanLimit


def schema() -> InMemorySchemaProvider:
    return InMemorySchemaProvider({"users": ["id", "name", "age"]})


class TestMinimalSelect:
    def test_select_star(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="users"),
            items=(SelectItem(expr=Wildcard()),),
        )
        p = plan(ast, schema())
        assert isinstance(p, Project)
        assert p.items == (ProjectionItem(expr=Wildcard(), alias=None),)
        assert isinstance(p.input, Scan)
        assert p.input.table == "users"

    def test_bare_column_resolves_to_qualified(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="users"),
            items=(SelectItem(expr=Column(None, "name")),),
        )
        p = plan(ast, schema())
        assert isinstance(p, Project)
        (item,) = p.items
        assert item.expr == Column(table="users", col="name")
        assert item.alias == "name"


class TestWhere:
    def test_where_becomes_filter(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="users"),
            items=(SelectItem(expr=Wildcard()),),
            where=BinaryExpr(
                op=BinaryOp.GT,
                left=Column(None, "age"),
                right=Literal(value=18),
            ),
        )
        p = plan(ast, schema())
        assert isinstance(p, Project)
        assert isinstance(p.input, Filter)
        assert p.input.predicate == BinaryExpr(
            op=BinaryOp.GT,
            left=Column("users", "age"),
            right=Literal(value=18),
        )


class TestOrderBy:
    def test_order_by_becomes_sort(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="users"),
            items=(SelectItem(expr=Wildcard()),),
            order_by=(AstSortKey(expr=Column(None, "age"), descending=True),),
        )
        p = plan(ast, schema())
        assert isinstance(p, Sort)
        (key,) = p.keys
        assert key.descending is True
        assert key.expr == Column("users", "age")


class TestLimitOffset:
    def test_limit_only(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="users"),
            items=(SelectItem(expr=Wildcard()),),
            limit=AstLimit(count=10),
        )
        p = plan(ast, schema())
        assert isinstance(p, PlanLimit)
        assert p.count == 10
        assert p.offset is None

    def test_offset_only(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="users"),
            items=(SelectItem(expr=Wildcard()),),
            limit=AstLimit(offset=5),
        )
        p = plan(ast, schema())
        assert isinstance(p, PlanLimit)
        assert p.offset == 5

    def test_empty_limit_not_emitted(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="users"),
            items=(SelectItem(expr=Wildcard()),),
            limit=AstLimit(),
        )
        p = plan(ast, schema())
        assert not isinstance(p, PlanLimit)


class TestDistinct:
    def test_distinct_wraps_project(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="users"),
            items=(SelectItem(expr=Column(None, "name")),),
            distinct=True,
        )
        p = plan(ast, schema())
        assert isinstance(p, Distinct)
        assert isinstance(p.input, Project)


class TestStackOrder:
    def test_distinct_sort_limit_order(self) -> None:
        # DISTINCT → Sort → Limit should nest outward in that order.
        ast = SelectStmt(
            from_=TableRef(table="users"),
            items=(SelectItem(expr=Column(None, "name")),),
            order_by=(AstSortKey(expr=Column(None, "name")),),
            limit=AstLimit(count=3),
            distinct=True,
        )
        p = plan(ast, schema())
        # Outermost: Limit
        assert isinstance(p, PlanLimit)
        # Next: Sort
        assert isinstance(p.input, Sort)
        # Next: Distinct
        assert isinstance(p.input.input, Distinct)
        # Next: Project
        assert isinstance(p.input.input.input, Project)


class TestAliasDerivation:
    def test_explicit_alias(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="users"),
            items=(SelectItem(expr=Column(None, "name"), alias="n"),),
        )
        p = plan(ast, schema())
        assert isinstance(p, Project)
        assert p.items[0].alias == "n"

    def test_bare_column_alias_is_col_name(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="users"),
            items=(SelectItem(expr=Column(None, "age")),),
        )
        p = plan(ast, schema())
        assert isinstance(p, Project)
        assert p.items[0].alias == "age"

    def test_function_call_alias(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="users"),
            items=(
                SelectItem(
                    expr=FunctionCall(
                        name="UPPER", args=(FuncArg(value=Column(None, "name")),)
                    )
                ),
            ),
        )
        p = plan(ast, schema())
        assert isinstance(p, Project)
        assert p.items[0].alias == "upper"

    def test_literal_no_alias(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="users"),
            items=(SelectItem(expr=Literal(value=42)),),
        )
        p = plan(ast, schema())
        assert isinstance(p, Project)
        assert p.items[0].alias is None


class TestExistsSubquery:
    def test_exists_in_where_resolves_inner_plan(self) -> None:
        """EXISTS (subquery) in WHERE: the inner SELECT is planned into a LogicalPlan."""
        sp = InMemorySchemaProvider({"t": ["x"], "s": ["y"]})
        inner = SelectStmt(
            from_=TableRef(table="s"),
            items=(SelectItem(expr=Literal(value=1)),),
        )
        outer = SelectStmt(
            from_=TableRef(table="t"),
            items=(SelectItem(expr=Wildcard()),),
            where=ExistsSubquery(query=inner),
        )
        p = plan(outer, sp)
        assert isinstance(p, Project)
        assert isinstance(p.input, Filter)
        pred = p.input.predicate
        assert isinstance(pred, ExistsSubquery)
        assert isinstance(pred.query, Project)  # inner was planned into a LogicalPlan

    def test_exists_in_select_list_resolves_inner_plan(self) -> None:
        """EXISTS used as a SELECT-list expression: inner plan is resolved."""
        sp = InMemorySchemaProvider({"t": ["x"], "s": ["y"]})
        inner = SelectStmt(
            from_=TableRef(table="s"),
            items=(SelectItem(expr=Wildcard()),),
        )
        outer = SelectStmt(
            from_=TableRef(table="t"),
            items=(SelectItem(expr=ExistsSubquery(query=inner)),),
        )
        p = plan(outer, sp)
        assert isinstance(p, Project)
        (item,) = p.items
        assert isinstance(item.expr, ExistsSubquery)
        assert isinstance(item.expr.query, Project)


class TestPlanAll:
    def test_plan_multiple(self) -> None:
        ast1 = SelectStmt(
            from_=TableRef(table="users"),
            items=(SelectItem(expr=Wildcard()),),
        )
        ast2 = SelectStmt(
            from_=TableRef(table="users"),
            items=(SelectItem(expr=Column(None, "id")),),
        )
        plans = plan_all([ast1, ast2], schema())
        assert len(plans) == 2
