"""JOIN planning — INNER / LEFT / CROSS / USING / NATURAL, nesting, ON validation."""

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
from sql_planner.errors import UnknownColumn


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


class TestJoinUsing:
    """JOIN … USING (col, …) — planner expands to explicit ON expression."""

    def test_using_single_column_produces_join_with_condition(self) -> None:
        # users.id = orders.user_id would be ON, but USING (id) would require
        # a shared column.  Use a schema where both tables share "id".
        sp = InMemorySchemaProvider({
            "a": ["id", "x"],
            "b": ["id", "y"],
        })
        ast = SelectStmt(
            from_=TableRef(table="a"),
            items=(SelectItem(expr=Wildcard()),),
            joins=(
                JoinClause(kind=JoinKind.INNER, right=TableRef(table="b"), using=("id",)),
            ),
        )
        p = plan(ast, sp)
        assert isinstance(p, Project)
        join = p.input
        assert isinstance(join, Join)
        assert join.kind == JoinKind.INNER
        # The condition must be a.id = b.id.
        cond = join.condition
        assert isinstance(cond, BinaryExpr)
        assert cond.op == BinaryOp.EQ
        assert isinstance(cond.left, Column) and cond.left.table == "a" and cond.left.col == "id"
        assert isinstance(cond.right, Column) and cond.right.table == "b" and cond.right.col == "id"

    def test_using_multi_column(self) -> None:
        sp = InMemorySchemaProvider({
            "a": ["k1", "k2", "va"],
            "b": ["k1", "k2", "vb"],
        })
        ast = SelectStmt(
            from_=TableRef(table="a"),
            items=(SelectItem(expr=Wildcard()),),
            joins=(
                JoinClause(kind=JoinKind.INNER, right=TableRef(table="b"), using=("k1", "k2")),
            ),
        )
        p = plan(ast, sp)
        join = p.input
        assert isinstance(join, Join)
        # Condition must be AND(a.k1 = b.k1, a.k2 = b.k2).
        cond = join.condition
        assert isinstance(cond, BinaryExpr) and cond.op == BinaryOp.AND
        left_eq, right_eq = cond.left, cond.right
        assert isinstance(left_eq, BinaryExpr) and left_eq.op == BinaryOp.EQ
        assert isinstance(right_eq, BinaryExpr) and right_eq.op == BinaryOp.EQ

    def test_using_three_table_chain(self) -> None:
        # In a chain a JOIN b USING (x) JOIN c USING (y), y lives in a, not b.
        # The planner must search the full accumulated scope to find the owner.
        sp = InMemorySchemaProvider({
            "orders": ["order_id", "cust_id", "prod_id"],
            "customers": ["cust_id", "cust_name"],
            "products": ["prod_id", "prod_name"],
        })
        ast = SelectStmt(
            from_=TableRef(table="orders"),
            items=(SelectItem(expr=Wildcard()),),
            joins=(
                JoinClause(
                    kind=JoinKind.INNER,
                    right=TableRef(table="customers"),
                    using=("cust_id",),
                ),
                JoinClause(
                    kind=JoinKind.INNER,
                    right=TableRef(table="products"),
                    using=("prod_id",),  # prod_id lives in orders, not customers
                ),
            ),
        )
        p = plan(ast, sp)
        # Should produce a nested join tree without raising UnknownColumn.
        assert isinstance(p, Project)
        outer = p.input
        assert isinstance(outer, Join)
        assert outer.kind == JoinKind.INNER
        inner = outer.left
        assert isinstance(inner, Join)
        # Inner join condition: orders.cust_id = customers.cust_id
        inner_cond = inner.condition
        assert isinstance(inner_cond, BinaryExpr) and inner_cond.op == BinaryOp.EQ
        # Outer join condition: orders.prod_id = products.prod_id
        outer_cond = outer.condition
        assert isinstance(outer_cond, BinaryExpr) and outer_cond.op == BinaryOp.EQ
        assert isinstance(outer_cond.left, Column) and outer_cond.left.table == "orders"
        assert outer_cond.left.col == "prod_id"

    def test_using_unknown_column_raises(self) -> None:
        sp = InMemorySchemaProvider({
            "a": ["id", "x"],
            "b": ["id", "y"],
        })
        ast = SelectStmt(
            from_=TableRef(table="a"),
            items=(SelectItem(expr=Wildcard()),),
            joins=(
                JoinClause(
                    kind=JoinKind.INNER,
                    right=TableRef(table="b"),
                    using=("nonexistent_col",),
                ),
            ),
        )
        with pytest.raises(UnknownColumn):
            plan(ast, sp)

    def test_using_left_join(self) -> None:
        sp = InMemorySchemaProvider({
            "a": ["id", "x"],
            "b": ["id", "y"],
        })
        ast = SelectStmt(
            from_=TableRef(table="a"),
            items=(SelectItem(expr=Wildcard()),),
            joins=(
                JoinClause(kind=JoinKind.LEFT, right=TableRef(table="b"), using=("id",)),
            ),
        )
        p = plan(ast, sp)
        join = p.input
        assert isinstance(join, Join)
        assert join.kind == JoinKind.LEFT
        assert join.condition is not None


class TestNaturalJoin:
    """NATURAL JOIN — planner finds shared columns from schema."""

    def test_natural_join_shared_column_becomes_inner_join(self) -> None:
        sp = InMemorySchemaProvider({
            "emp": ["id", "name", "dept_id"],
            "dept": ["dept_id", "dept_name"],
        })
        ast = SelectStmt(
            from_=TableRef(table="emp"),
            items=(SelectItem(expr=Wildcard()),),
            joins=(
                JoinClause(kind=JoinKind.NATURAL, right=TableRef(table="dept"), on=None),
            ),
        )
        p = plan(ast, sp)
        join = p.input
        assert isinstance(join, Join)
        assert join.kind == JoinKind.INNER
        # Condition: emp.dept_id = dept.dept_id
        cond = join.condition
        assert isinstance(cond, BinaryExpr) and cond.op == BinaryOp.EQ

    def test_natural_join_no_shared_cols_becomes_cross(self) -> None:
        sp = InMemorySchemaProvider({
            "a": ["x", "y"],
            "b": ["p", "q"],
        })
        ast = SelectStmt(
            from_=TableRef(table="a"),
            items=(SelectItem(expr=Wildcard()),),
            joins=(
                JoinClause(kind=JoinKind.NATURAL, right=TableRef(table="b"), on=None),
            ),
        )
        p = plan(ast, sp)
        join = p.input
        assert isinstance(join, Join)
        assert join.kind == JoinKind.CROSS
        assert join.condition is None

    def test_natural_join_multiple_shared_cols(self) -> None:
        sp = InMemorySchemaProvider({
            "a": ["k1", "k2", "va"],
            "b": ["k1", "k2", "vb"],
        })
        ast = SelectStmt(
            from_=TableRef(table="a"),
            items=(SelectItem(expr=Wildcard()),),
            joins=(
                JoinClause(kind=JoinKind.NATURAL, right=TableRef(table="b"), on=None),
            ),
        )
        p = plan(ast, sp)
        join = p.input
        assert isinstance(join, Join)
        assert join.kind == JoinKind.INNER
        # Condition: AND(a.k1 = b.k1, a.k2 = b.k2)
        cond = join.condition
        assert isinstance(cond, BinaryExpr) and cond.op == BinaryOp.AND
