"""JOIN planning — INNER / LEFT / CROSS, nesting, ON validation."""

from __future__ import annotations

import pytest

from sql_planner import (
    BinaryExpr,
    BinaryOp,
    Column,
    InMemorySchemaProvider,
    Join,
    JoinClause,
    JoinKind,
    Project,
    Scan,
    SelectItem,
    SelectStmt,
    TableRef,
    UnsupportedStatement,
    Wildcard,
    plan,
)


def schema() -> InMemorySchemaProvider:
    return InMemorySchemaProvider({
        "users": ["id", "name"],
        "orders": ["id", "user_id", "total"],
        "items": ["id", "order_id"],
    })


class TestInnerJoin:
    def test_simple_inner_join(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="users", alias="u"),
            items=(SelectItem(expr=Wildcard()),),
            joins=(
                JoinClause(
                    kind=JoinKind.INNER,
                    right=TableRef(table="orders", alias="o"),
                    on=BinaryExpr(
                        op=BinaryOp.EQ,
                        left=Column("u", "id"),
                        right=Column("o", "user_id"),
                    ),
                ),
            ),
        )
        p = plan(ast, schema())
        assert isinstance(p, Project)
        assert isinstance(p.input, Join)
        assert p.input.kind == JoinKind.INNER
        assert isinstance(p.input.left, Scan)
        assert isinstance(p.input.right, Scan)
        assert p.input.left.alias == "u"
        assert p.input.right.alias == "o"


class TestLeftJoin:
    def test_left_join(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="users"),
            items=(SelectItem(expr=Wildcard()),),
            joins=(
                JoinClause(
                    kind=JoinKind.LEFT,
                    right=TableRef(table="orders"),
                    on=BinaryExpr(
                        op=BinaryOp.EQ,
                        left=Column("users", "id"),
                        right=Column("orders", "user_id"),
                    ),
                ),
            ),
        )
        p = plan(ast, schema())
        assert isinstance(p, Project)
        assert isinstance(p.input, Join)
        assert p.input.kind == JoinKind.LEFT


class TestCrossJoin:
    def test_cross_with_no_on(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="users"),
            items=(SelectItem(expr=Wildcard()),),
            joins=(
                JoinClause(kind=JoinKind.CROSS, right=TableRef(table="orders"), on=None),
            ),
        )
        p = plan(ast, schema())
        assert isinstance(p, Project)
        assert isinstance(p.input, Join)
        assert p.input.kind == JoinKind.CROSS
        assert p.input.condition is None

    def test_cross_with_on_rejected(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="users"),
            items=(SelectItem(expr=Wildcard()),),
            joins=(
                JoinClause(
                    kind=JoinKind.CROSS,
                    right=TableRef(table="orders"),
                    on=BinaryExpr(
                        op=BinaryOp.EQ,
                        left=Column("users", "id"),
                        right=Column("orders", "user_id"),
                    ),
                ),
            ),
        )
        with pytest.raises(UnsupportedStatement):
            plan(ast, schema())

    def test_inner_without_on_rejected(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="users"),
            items=(SelectItem(expr=Wildcard()),),
            joins=(
                JoinClause(kind=JoinKind.INNER, right=TableRef(table="orders"), on=None),
            ),
        )
        with pytest.raises(UnsupportedStatement):
            plan(ast, schema())


class TestNestedJoin:
    def test_three_way_join_left_associative(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="users"),
            items=(SelectItem(expr=Wildcard()),),
            joins=(
                JoinClause(
                    kind=JoinKind.INNER,
                    right=TableRef(table="orders"),
                    on=BinaryExpr(
                        op=BinaryOp.EQ,
                        left=Column("users", "id"),
                        right=Column("orders", "user_id"),
                    ),
                ),
                JoinClause(
                    kind=JoinKind.INNER,
                    right=TableRef(table="items"),
                    on=BinaryExpr(
                        op=BinaryOp.EQ,
                        left=Column("orders", "id"),
                        right=Column("items", "order_id"),
                    ),
                ),
            ),
        )
        p = plan(ast, schema())
        assert isinstance(p, Project)
        # Outer join is the second one (items); left-associative.
        assert isinstance(p.input, Join)
        assert isinstance(p.input.right, Scan)
        assert p.input.right.table == "items"
        assert isinstance(p.input.left, Join)
        assert p.input.left.right.table == "orders"


class TestDuplicateAliasRejected:
    def test_duplicate_alias(self) -> None:
        ast = SelectStmt(
            from_=TableRef(table="users", alias="u"),
            items=(SelectItem(expr=Wildcard()),),
            joins=(
                JoinClause(
                    kind=JoinKind.CROSS,
                    right=TableRef(table="orders", alias="u"),
                    on=None,
                ),
            ),
        )
        with pytest.raises(UnsupportedStatement):
            plan(ast, schema())
