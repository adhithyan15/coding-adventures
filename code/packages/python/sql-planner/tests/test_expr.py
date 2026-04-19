"""Expression IR — construction, equality, and helpers."""

from __future__ import annotations

import pytest

from sql_planner.expr import (
    AggFunc,
    AggregateExpr,
    Between,
    BinaryExpr,
    BinaryOp,
    Column,
    FuncArg,
    FunctionCall,
    In,
    IsNotNull,
    IsNull,
    Like,
    Literal,
    NotIn,
    NotLike,
    UnaryExpr,
    UnaryOp,
    Wildcard,
    collect_columns,
    contains_aggregate,
)


class TestLiteralEquality:
    def test_same_value_equal(self) -> None:
        assert Literal(value=42) == Literal(value=42)

    def test_different_values_unequal(self) -> None:
        assert Literal(value=42) != Literal(value=43)

    def test_frozen(self) -> None:
        lit = Literal(value=42)
        with pytest.raises(Exception):  # noqa: B017 — FrozenInstanceError
            lit.value = 99  # type: ignore[misc]


class TestColumn:
    def test_bare(self) -> None:
        assert Column(table=None, col="x") == Column(table=None, col="x")

    def test_qualified(self) -> None:
        assert Column(table="t", col="x") != Column(table="u", col="x")


class TestFuncArg:
    def test_star(self) -> None:
        a = FuncArg(star=True)
        assert a.star is True
        assert a.value is None

    def test_value(self) -> None:
        a = FuncArg(value=Literal(value=1))
        assert a.star is False

    def test_both_rejected(self) -> None:
        with pytest.raises(ValueError):
            FuncArg(star=True, value=Literal(value=1))

    def test_neither_rejected(self) -> None:
        with pytest.raises(ValueError):
            FuncArg()


class TestContainsAggregate:
    def test_literal_no(self) -> None:
        assert not contains_aggregate(Literal(value=1))

    def test_column_no(self) -> None:
        assert not contains_aggregate(Column(table=None, col="x"))

    def test_top_level_agg(self) -> None:
        e = AggregateExpr(func=AggFunc.COUNT, arg=FuncArg(star=True))
        assert contains_aggregate(e)

    def test_nested_agg(self) -> None:
        # SUM(x) * 2
        inner = AggregateExpr(
            func=AggFunc.SUM, arg=FuncArg(value=Column(table=None, col="x"))
        )
        outer = BinaryExpr(op=BinaryOp.MUL, left=inner, right=Literal(value=2))
        assert contains_aggregate(outer)

    def test_in_function_arg(self) -> None:
        inner = AggregateExpr(func=AggFunc.MIN, arg=FuncArg(value=Literal(value=1)))
        fc = FunctionCall(name="UPPER", args=(FuncArg(value=inner),))
        assert contains_aggregate(fc)

    def test_in_between(self) -> None:
        hi = AggregateExpr(
            func=AggFunc.MAX, arg=FuncArg(value=Column(table=None, col="y"))
        )
        e = Between(
            operand=Column(table=None, col="x"),
            low=Literal(value=1),
            high=hi,
        )
        assert contains_aggregate(e)

    def test_in_in_list(self) -> None:
        agg = AggregateExpr(func=AggFunc.MAX, arg=FuncArg(value=Literal(value=2)))
        e = In(
            operand=Column(table=None, col="x"),
            values=(Literal(value=1), agg),
        )
        assert contains_aggregate(e)

    def test_in_not_in(self) -> None:
        e = NotIn(
            operand=AggregateExpr(func=AggFunc.COUNT, arg=FuncArg(star=True)),
            values=(Literal(value=1),),
        )
        assert contains_aggregate(e)

    def test_in_like(self) -> None:
        e = Like(
            operand=AggregateExpr(func=AggFunc.MIN, arg=FuncArg(value=Literal(value=1))),
            pattern="%",
        )
        assert contains_aggregate(e)

    def test_in_not_like(self) -> None:
        e = NotLike(operand=Column(table=None, col="x"), pattern="%")
        assert not contains_aggregate(e)

    def test_in_is_null(self) -> None:
        e = IsNull(operand=AggregateExpr(func=AggFunc.SUM, arg=FuncArg(value=Literal(value=1))))
        assert contains_aggregate(e)

    def test_in_is_not_null(self) -> None:
        e = IsNotNull(operand=Column(table=None, col="x"))
        assert not contains_aggregate(e)

    def test_unary(self) -> None:
        agg = AggregateExpr(func=AggFunc.COUNT, arg=FuncArg(star=True))
        e = UnaryExpr(op=UnaryOp.NOT, operand=agg)
        assert contains_aggregate(e)

    def test_wildcard_no(self) -> None:
        assert not contains_aggregate(Wildcard())


class TestCollectColumns:
    def test_literal_empty(self) -> None:
        assert collect_columns(Literal(value=1)) == []

    def test_single_column(self) -> None:
        c = Column(table="t", col="x")
        assert collect_columns(c) == [c]

    def test_binary(self) -> None:
        left = Column(table=None, col="x")
        right = Column(table=None, col="y")
        e = BinaryExpr(op=BinaryOp.EQ, left=left, right=right)
        assert collect_columns(e) == [left, right]

    def test_aggregate_inside(self) -> None:
        c = Column(table=None, col="x")
        e = AggregateExpr(func=AggFunc.SUM, arg=FuncArg(value=c))
        assert collect_columns(e) == [c]

    def test_aggregate_star(self) -> None:
        e = AggregateExpr(func=AggFunc.COUNT, arg=FuncArg(star=True))
        assert collect_columns(e) == []

    def test_function_call(self) -> None:
        c = Column(table=None, col="x")
        fc = FunctionCall(name="UPPER", args=(FuncArg(value=c),))
        assert collect_columns(fc) == [c]

    def test_is_null_and_is_not_null(self) -> None:
        c = Column(table=None, col="x")
        assert collect_columns(IsNull(operand=c)) == [c]
        assert collect_columns(IsNotNull(operand=c)) == [c]

    def test_between(self) -> None:
        a = Column(table=None, col="a")
        b = Column(table=None, col="b")
        cc = Column(table=None, col="c")
        e = Between(operand=a, low=b, high=cc)
        assert collect_columns(e) == [a, b, cc]

    def test_in(self) -> None:
        a, b = Column(table=None, col="a"), Column(table=None, col="b")
        assert collect_columns(In(operand=a, values=(b,))) == [a, b]

    def test_not_in(self) -> None:
        a, b = Column(table=None, col="a"), Column(table=None, col="b")
        assert collect_columns(NotIn(operand=a, values=(b,))) == [a, b]

    def test_like(self) -> None:
        c = Column(table=None, col="x")
        assert collect_columns(Like(operand=c, pattern="%")) == [c]

    def test_not_like(self) -> None:
        c = Column(table=None, col="x")
        assert collect_columns(NotLike(operand=c, pattern="%")) == [c]

    def test_unary(self) -> None:
        c = Column(table=None, col="x")
        assert collect_columns(UnaryExpr(op=UnaryOp.NEG, operand=c)) == [c]

    def test_wildcard_empty(self) -> None:
        assert collect_columns(Wildcard()) == []
