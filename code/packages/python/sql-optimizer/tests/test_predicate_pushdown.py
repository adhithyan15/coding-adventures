"""PredicatePushdown — through Project, through Join, AND-splitting, HAVING blocked."""

from __future__ import annotations

from sql_planner import (
    AggFunc,
    Aggregate,
    AggregateItem,
    BinaryExpr,
    BinaryOp,
    Column,
    Filter,
    FuncArg,
    Having,
    Join,
    JoinKind,
    Literal,
    LogicalPlan,
    Project,
    ProjectionItem,
    Scan,
    Sort,
)
from sql_planner.plan import Limit, SortKey

from sql_optimizer import PredicatePushdown


def pp(plan: LogicalPlan) -> LogicalPlan:
    return PredicatePushdown()(plan)


class TestThroughProject:
    def test_filter_pushes_below_project(self) -> None:
        tree = Filter(
            input=Project(
                input=Scan(table="t"),
                items=(ProjectionItem(expr=Column("t", "name"), alias="name"),),
            ),
            predicate=BinaryExpr(
                op=BinaryOp.GT, left=Column("t", "age"), right=Literal(18)
            ),
        )
        out = pp(tree)
        # Outer = Project, inner = Filter, innermost = Scan.
        assert isinstance(out, Project)
        assert isinstance(out.input, Filter)
        assert isinstance(out.input.input, Scan)


class TestAndSplit:
    def test_split_and_push_to_correct_sides(self) -> None:
        tree = Filter(
            input=Join(
                left=Scan(table="employees", alias="e"),
                right=Scan(table="departments", alias="d"),
                kind=JoinKind.INNER,
                condition=BinaryExpr(
                    op=BinaryOp.EQ,
                    left=Column("e", "dept_id"),
                    right=Column("d", "id"),
                ),
            ),
            predicate=BinaryExpr(
                op=BinaryOp.AND,
                left=BinaryExpr(
                    op=BinaryOp.EQ,
                    left=Column("e", "active"),
                    right=Literal(True),
                ),
                right=BinaryExpr(
                    op=BinaryOp.EQ,
                    left=Column("d", "name"),
                    right=Literal("Engineering"),
                ),
            ),
        )
        out = pp(tree)
        assert isinstance(out, Join)
        # Left child should be Filter(active=TRUE) over Scan(employees).
        assert isinstance(out.left, Filter)
        assert isinstance(out.left.input, Scan)
        # Right child should be Filter(name=Engineering) over Scan(departments).
        assert isinstance(out.right, Filter)
        assert isinstance(out.right.input, Scan)

    def test_conjunct_referencing_both_sides_stays(self) -> None:
        tree = Filter(
            input=Join(
                left=Scan(table="a"),
                right=Scan(table="b"),
                kind=JoinKind.INNER,
                condition=BinaryExpr(
                    op=BinaryOp.EQ, left=Column("a", "k"), right=Column("b", "k")
                ),
            ),
            predicate=BinaryExpr(
                op=BinaryOp.AND,
                left=BinaryExpr(
                    op=BinaryOp.EQ, left=Column("a", "x"), right=Literal(1)
                ),
                right=BinaryExpr(
                    op=BinaryOp.GT, left=Column("a", "x"), right=Column("b", "y")
                ),
            ),
        )
        out = pp(tree)
        # Top is a Filter wrapping a Join, because one conjunct spans both sides.
        assert isinstance(out, Filter)
        assert isinstance(out.input, Join)
        # The left side should have the ``a.x = 1`` filter pushed.
        assert isinstance(out.input.left, Filter)


class TestHavingBlocked:
    def test_filter_above_aggregate_does_not_push(self) -> None:
        agg = Aggregate(
            input=Scan(table="t"),
            group_by=(Column("t", "region"),),
            aggregates=(
                AggregateItem(
                    func=AggFunc.SUM,
                    arg=FuncArg(value=Column("t", "amount")),
                    alias="total",
                ),
            ),
        )
        tree = Having(
            input=agg,
            predicate=BinaryExpr(
                op=BinaryOp.GT, left=Column("t", "region"), right=Literal(0)
            ),
        )
        out = pp(tree)
        assert isinstance(out, Having)
        # Aggregate's input is still the Scan — no filter pushed into it.
        assert isinstance(out.input, Aggregate)
        assert isinstance(out.input.input, Scan)


class TestLimitBlocked:
    def test_filter_above_limit_does_not_push(self) -> None:
        tree = Filter(
            input=Limit(input=Scan(table="t"), count=5),
            predicate=BinaryExpr(
                op=BinaryOp.EQ, left=Column("t", "x"), right=Literal(1)
            ),
        )
        out = pp(tree)
        assert isinstance(out, Filter)
        assert isinstance(out.input, Limit)


class TestThroughSort:
    def test_pushes_through_sort(self) -> None:
        tree = Filter(
            input=Sort(
                input=Scan(table="t"),
                keys=(SortKey(expr=Column("t", "x")),),
            ),
            predicate=BinaryExpr(
                op=BinaryOp.EQ, left=Column("t", "y"), right=Literal(1)
            ),
        )
        out = pp(tree)
        assert isinstance(out, Sort)
        assert isinstance(out.input, Filter)


class TestIdempotent:
    def test_twice_is_same(self) -> None:
        tree = Filter(
            input=Project(
                input=Scan(table="t"),
                items=(ProjectionItem(expr=Column("t", "x"), alias="x"),),
            ),
            predicate=BinaryExpr(
                op=BinaryOp.EQ, left=Column("t", "x"), right=Literal(1)
            ),
        )
        once = pp(tree)
        twice = pp(once)
        assert once == twice
