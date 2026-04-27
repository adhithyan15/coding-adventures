"""DML planning — INSERT VALUES, UPDATE, DELETE."""

from __future__ import annotations

import pytest

from sql_planner import (
    AggFunc,
    AggregateExpr,
    Assignment,
    BinaryExpr,
    BinaryOp,
    Column,
    DeleteStmt,
    FuncArg,
    InMemorySchemaProvider,
    InsertValuesStmt,
    InvalidAggregate,
    Literal,
    UnknownColumn,
    UpdateStmt,
    plan,
)
from sql_planner.plan import Delete, Insert, Update


def schema() -> InMemorySchemaProvider:
    return InMemorySchemaProvider({"users": ["id", "name", "age"]})


class TestInsert:
    def test_with_column_list(self) -> None:
        ast = InsertValuesStmt(
            table="users",
            columns=("id", "name"),
            rows=((Literal(value=1), Literal(value="alice")),),
        )
        p = plan(ast, schema())
        assert isinstance(p, Insert)
        assert p.table == "users"
        assert p.columns == ("id", "name")
        assert p.source.values == ((Literal(value=1), Literal(value="alice")),)

    def test_without_column_list(self) -> None:
        ast = InsertValuesStmt(
            table="users",
            columns=None,
            rows=((Literal(value=1), Literal(value="a"), Literal(value=30)),),
        )
        p = plan(ast, schema())
        assert isinstance(p, Insert)
        assert p.columns is None

    def test_unknown_column(self) -> None:
        ast = InsertValuesStmt(
            table="users",
            columns=("id", "bogus"),
            rows=((Literal(value=1), Literal(value=2)),),
        )
        with pytest.raises(UnknownColumn):
            plan(ast, schema())


class TestUpdate:
    def test_simple_update(self) -> None:
        ast = UpdateStmt(
            table="users",
            assignments=(Assignment(column="name", value=Literal(value="bob")),),
            where=BinaryExpr(
                op=BinaryOp.EQ, left=Column(None, "id"), right=Literal(value=1)
            ),
        )
        p = plan(ast, schema())
        assert isinstance(p, Update)
        assert p.table == "users"
        assert p.assignments[0].column == "name"
        assert p.predicate == BinaryExpr(
            op=BinaryOp.EQ, left=Column("users", "id"), right=Literal(value=1)
        )

    def test_update_without_where(self) -> None:
        ast = UpdateStmt(
            table="users",
            assignments=(Assignment(column="age", value=Literal(value=0)),),
        )
        p = plan(ast, schema())
        assert isinstance(p, Update)
        assert p.predicate is None

    def test_unknown_column_in_set(self) -> None:
        ast = UpdateStmt(
            table="users",
            assignments=(Assignment(column="bogus", value=Literal(value=1)),),
        )
        with pytest.raises(UnknownColumn):
            plan(ast, schema())

    def test_agg_in_where_rejected(self) -> None:
        ast = UpdateStmt(
            table="users",
            assignments=(Assignment(column="age", value=Literal(value=0)),),
            where=BinaryExpr(
                op=BinaryOp.GT,
                left=AggregateExpr(func=AggFunc.COUNT, arg=FuncArg(star=True)),
                right=Literal(value=0),
            ),
        )
        with pytest.raises(InvalidAggregate):
            plan(ast, schema())


class TestDelete:
    def test_delete_with_where(self) -> None:
        ast = DeleteStmt(
            table="users",
            where=BinaryExpr(
                op=BinaryOp.EQ, left=Column(None, "id"), right=Literal(value=1)
            ),
        )
        p = plan(ast, schema())
        assert isinstance(p, Delete)
        assert p.table == "users"
        assert p.predicate == BinaryExpr(
            op=BinaryOp.EQ, left=Column("users", "id"), right=Literal(value=1)
        )

    def test_delete_without_where(self) -> None:
        ast = DeleteStmt(table="users")
        p = plan(ast, schema())
        assert isinstance(p, Delete)
        assert p.predicate is None

    def test_agg_in_where_rejected(self) -> None:
        ast = DeleteStmt(
            table="users",
            where=BinaryExpr(
                op=BinaryOp.GT,
                left=AggregateExpr(func=AggFunc.COUNT, arg=FuncArg(star=True)),
                right=Literal(value=0),
            ),
        )
        with pytest.raises(InvalidAggregate):
            plan(ast, schema())
