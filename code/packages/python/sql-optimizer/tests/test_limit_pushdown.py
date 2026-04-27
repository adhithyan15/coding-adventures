"""LimitPushdown — scan_limit annotation through safe operators."""

from __future__ import annotations

from sql_planner import (
    BinaryExpr,
    BinaryOp,
    Column,
    Filter,
    Literal,
    LogicalPlan,
    Project,
    ProjectionItem,
    Scan,
    Sort,
)
from sql_planner.plan import Limit, SortKey

from sql_optimizer import LimitPushdown


def lp(plan: LogicalPlan) -> LogicalPlan:
    return LimitPushdown()(plan)


class TestThroughProject:
    def test_limit_annotates_scan_through_project(self) -> None:
        tree = Limit(
            input=Project(
                input=Scan(table="t"),
                items=(ProjectionItem(expr=Column("t", "x"), alias="x"),),
            ),
            count=5,
        )
        out = lp(tree)
        assert isinstance(out, Limit)
        scan = out.input.input
        assert scan.scan_limit == 5


class TestThroughFilter:
    def test_limit_hint_through_filter(self) -> None:
        tree = Limit(
            input=Filter(
                input=Scan(table="t"),
                predicate=BinaryExpr(
                    op=BinaryOp.EQ, left=Column("t", "x"), right=Literal(1)
                ),
            ),
            count=10,
        )
        out = lp(tree)
        # Filter is inside Limit; scan under filter gets the hint.
        scan = out.input.input
        assert scan.scan_limit == 10


class TestThroughSortBlocked:
    def test_limit_does_not_push_through_sort(self) -> None:
        tree = Limit(
            input=Sort(
                input=Scan(table="t"),
                keys=(SortKey(expr=Column("t", "x")),),
            ),
            count=5,
        )
        out = lp(tree)
        # Scan should remain unannotated.
        scan = out.input.input
        assert scan.scan_limit is None


class TestOffsetBlocksPush:
    def test_offset_prevents_push(self) -> None:
        tree = Limit(
            input=Project(
                input=Scan(table="t"),
                items=(ProjectionItem(expr=Column("t", "x"), alias="x"),),
            ),
            count=5,
            offset=10,
        )
        out = lp(tree)
        scan = out.input.input
        assert scan.scan_limit is None


class TestIdempotent:
    def test_twice_same(self) -> None:
        tree = Limit(
            input=Project(
                input=Scan(table="t"),
                items=(ProjectionItem(expr=Column("t", "x"), alias="x"),),
            ),
            count=5,
        )
        once = lp(tree)
        twice = lp(once)
        assert once == twice
