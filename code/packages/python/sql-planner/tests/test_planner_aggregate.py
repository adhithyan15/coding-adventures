"""GROUP BY, HAVING, implicit aggregation, aggregate-in-WHERE rejection."""

from __future__ import annotations

import pytest

from sql_planner import (
    AggFunc,
    Aggregate,
    AggregateExpr,
    BinaryExpr,
    BinaryOp,
    Column,
    Filter,
    FuncArg,
    Having,
    InMemorySchemaProvider,
    InvalidAggregate,
    Literal,
    Project,
    Scan,
    SelectItem,
    SelectStmt,
    TableRef,
    plan,
)


def schema() -> InMemorySchemaProvider:
    return InMemorySchemaProvider({"sales": ["region", "amount"]})


class TestImplicitAggregate:
    def test_count_star_no_group_by(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="sales"),
            items=(
                SelectItem(
                    expr=AggregateExpr(func=AggFunc.COUNT, arg=FuncArg(star=True))
                ),
            ),
        )
        p = plan(ast, schema())
        assert isinstance(p, Project)
        assert isinstance(p.input, Aggregate)
        assert p.input.group_by == ()
        assert len(p.input.aggregates) == 1
        assert p.input.aggregates[0].func == AggFunc.COUNT


class TestGroupBy:
    def test_group_by_emits_aggregate(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="sales"),
            items=(
                SelectItem(expr=Column(None, "region")),
                SelectItem(
                    expr=AggregateExpr(
                        func=AggFunc.SUM, arg=FuncArg(value=Column(None, "amount"))
                    )
                ),
            ),
            group_by=(Column(None, "region"),),
        )
        p = plan(ast, schema())
        assert isinstance(p, Project)
        assert isinstance(p.input, Aggregate)
        assert p.input.group_by == (Column("sales", "region"),)
        assert len(p.input.aggregates) == 1


class TestHaving:
    def test_having_emits_having_node(self) -> None:
        agg = AggregateExpr(
            func=AggFunc.SUM, arg=FuncArg(value=Column(None, "amount"))
        )
        ast = SelectStmt(
            from_=TableRef(table="sales"),
            items=(
                SelectItem(expr=Column(None, "region")),
                SelectItem(expr=agg),
            ),
            group_by=(Column(None, "region"),),
            having=BinaryExpr(op=BinaryOp.GT, left=agg, right=Literal(value=100)),
        )
        p = plan(ast, schema())
        assert isinstance(p, Project)
        assert isinstance(p.input, Having)
        assert isinstance(p.input.input, Aggregate)

    def test_having_alone_triggers_aggregate(self) -> None:
        # HAVING that references an aggregate should trigger Aggregate even
        # with no GROUP BY.
        agg = AggregateExpr(func=AggFunc.COUNT, arg=FuncArg(star=True))
        ast = SelectStmt(
            from_=TableRef(table="sales"),
            items=(SelectItem(expr=Literal(value=1)),),
            having=BinaryExpr(op=BinaryOp.GT, left=agg, right=Literal(value=0)),
        )
        p = plan(ast, schema())
        assert isinstance(p, Project)
        assert isinstance(p.input, Having)
        assert isinstance(p.input.input, Aggregate)


class TestAggregateInWhereRejected:
    def test_agg_in_where_raises(self) -> None:
        agg = AggregateExpr(func=AggFunc.COUNT, arg=FuncArg(star=True))
        ast = SelectStmt(
            from_=TableRef(table="sales"),
            items=(SelectItem(expr=Column(None, "region")),),
            where=BinaryExpr(op=BinaryOp.GT, left=agg, right=Literal(value=0)),
        )
        with pytest.raises(InvalidAggregate):
            plan(ast, schema())


class TestNoAggregateNoGroupBy:
    def test_non_agg_select_does_not_emit_aggregate(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="sales"),
            items=(SelectItem(expr=Column(None, "region")),),
        )
        p = plan(ast, schema())
        assert isinstance(p, Project)
        assert isinstance(p.input, Scan)

    def test_where_without_agg(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="sales"),
            items=(SelectItem(expr=Column(None, "region")),),
            where=BinaryExpr(
                op=BinaryOp.GT,
                left=Column(None, "amount"),
                right=Literal(value=0),
            ),
        )
        p = plan(ast, schema())
        assert isinstance(p, Project)
        assert isinstance(p.input, Filter)


class TestAggregateDistinct:
    def test_count_distinct_preserved(self) -> None:
        agg = AggregateExpr(
            func=AggFunc.COUNT,
            arg=FuncArg(value=Column(None, "region")),
            distinct=True,
        )
        ast = SelectStmt(
            from_=TableRef(table="sales"),
            items=(SelectItem(expr=agg),),
        )
        p = plan(ast, schema())
        assert isinstance(p, Project)
        assert isinstance(p.input, Aggregate)
        assert p.input.aggregates[0].distinct is True
