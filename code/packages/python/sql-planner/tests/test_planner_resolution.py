"""Column resolution — ambiguity, unknown, qualified references, nested expressions."""

from __future__ import annotations

import pytest

from sql_planner import (
    AmbiguousColumn,
    Between,
    BinaryExpr,
    BinaryOp,
    Column,
    Expr,
    FuncArg,
    FunctionCall,
    In,
    InMemorySchemaProvider,
    IsNotNull,
    IsNull,
    JoinClause,
    JoinKind,
    Like,
    Literal,
    LogicalPlan,
    NotIn,
    NotLike,
    Project,
    SelectItem,
    SelectStmt,
    TableRef,
    UnaryExpr,
    UnaryOp,
    UnknownColumn,
    UnknownTable,
    Wildcard,
    plan,
)


def two_table_schema() -> InMemorySchemaProvider:
    return InMemorySchemaProvider({
        "a": ["id", "x"],
        "b": ["id", "y"],
    })


class TestBareColumnQualification:
    def test_unambiguous_bare_column(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="a"),
            joins=(
                JoinClause(
                    kind=JoinKind.CROSS, right=TableRef(table="b"), on=None
                ),
            ),
            items=(SelectItem(expr=Column(None, "x")),),
        )
        p = plan(ast, two_table_schema())
        assert isinstance(p, Project)
        assert p.items[0].expr == Column("a", "x")


class TestAmbiguous:
    def test_ambiguous_raises(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="a"),
            joins=(
                JoinClause(
                    kind=JoinKind.CROSS, right=TableRef(table="b"), on=None
                ),
            ),
            items=(SelectItem(expr=Column(None, "id")),),
        )
        with pytest.raises(AmbiguousColumn) as ei:
            plan(ast, two_table_schema())
        assert ei.value.column == "id"
        assert set(ei.value.tables) == {"a", "b"}


class TestUnknownColumn:
    def test_bare_unknown(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="a"),
            items=(SelectItem(expr=Column(None, "nope")),),
        )
        with pytest.raises(UnknownColumn):
            plan(ast, two_table_schema())

    def test_qualified_unknown_column(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="a"),
            items=(SelectItem(expr=Column("a", "nope")),),
        )
        with pytest.raises(UnknownColumn):
            plan(ast, two_table_schema())

    def test_qualified_unknown_table(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="a"),
            items=(SelectItem(expr=Column("c", "x")),),
        )
        with pytest.raises(UnknownColumn):
            plan(ast, two_table_schema())


class TestUnknownTableInFrom:
    def test_from_unknown(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="missing"),
            items=(SelectItem(expr=Wildcard()),),
        )
        with pytest.raises(UnknownTable):
            plan(ast, two_table_schema())


class TestAliasScopes:
    def test_alias_qualifies_column(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="a", alias="aa"),
            items=(SelectItem(expr=Column("aa", "x")),),
        )
        schema = InMemorySchemaProvider({"a": ["x"]})
        p = plan(ast, schema)
        assert isinstance(p, Project)
        assert p.items[0].expr == Column("aa", "x")


class TestResolutionRecursesIntoExprs:
    def _one_col(self) -> InMemorySchemaProvider:
        return InMemorySchemaProvider({"t": ["x", "y"]})

    def _plan_where(self, where: Expr) -> LogicalPlan:
        ast = SelectStmt(
            from_=TableRef(table="t"),
            items=(SelectItem(expr=Wildcard()),),
            where=where,
        )
        return plan(ast, self._one_col())

    def test_binary(self) -> None:
        p = self._plan_where(
            BinaryExpr(op=BinaryOp.EQ, left=Column(None, "x"), right=Column(None, "y"))
        )
        assert isinstance(p, Project)
        f = p.input
        assert f.predicate == BinaryExpr(
            op=BinaryOp.EQ, left=Column("t", "x"), right=Column("t", "y")
        )

    def test_unary(self) -> None:
        p = self._plan_where(UnaryExpr(op=UnaryOp.NOT, operand=Column(None, "x")))
        f = p.input
        assert f.predicate == UnaryExpr(op=UnaryOp.NOT, operand=Column("t", "x"))

    def test_function_call(self) -> None:
        p = self._plan_where(
            FunctionCall(name="LEN", args=(FuncArg(value=Column(None, "x")),))
        )
        f = p.input
        assert f.predicate == FunctionCall(
            name="LEN", args=(FuncArg(value=Column("t", "x")),)
        )

    def test_function_star_arg_passes_through(self) -> None:
        p = self._plan_where(
            FunctionCall(name="F", args=(FuncArg(star=True),))
        )
        f = p.input
        assert f.predicate == FunctionCall(name="F", args=(FuncArg(star=True),))

    def test_is_null(self) -> None:
        p = self._plan_where(IsNull(operand=Column(None, "x")))
        f = p.input
        assert f.predicate == IsNull(operand=Column("t", "x"))

    def test_is_not_null(self) -> None:
        p = self._plan_where(IsNotNull(operand=Column(None, "x")))
        f = p.input
        assert f.predicate == IsNotNull(operand=Column("t", "x"))

    def test_between(self) -> None:
        p = self._plan_where(
            Between(
                operand=Column(None, "x"),
                low=Literal(value=1),
                high=Column(None, "y"),
            )
        )
        f = p.input
        assert f.predicate == Between(
            operand=Column("t", "x"),
            low=Literal(value=1),
            high=Column("t", "y"),
        )

    def test_in(self) -> None:
        p = self._plan_where(
            In(operand=Column(None, "x"), values=(Literal(value=1), Column(None, "y")))
        )
        f = p.input
        assert f.predicate == In(
            operand=Column("t", "x"),
            values=(Literal(value=1), Column("t", "y")),
        )

    def test_not_in(self) -> None:
        p = self._plan_where(
            NotIn(operand=Column(None, "x"), values=(Literal(value=1),))
        )
        f = p.input
        assert f.predicate == NotIn(
            operand=Column("t", "x"), values=(Literal(value=1),)
        )

    def test_like(self) -> None:
        p = self._plan_where(Like(operand=Column(None, "x"), pattern="%a%"))
        f = p.input
        assert f.predicate == Like(operand=Column("t", "x"), pattern="%a%")

    def test_not_like(self) -> None:
        p = self._plan_where(NotLike(operand=Column(None, "x"), pattern="%a%"))
        f = p.input
        assert f.predicate == NotLike(operand=Column("t", "x"), pattern="%a%")
