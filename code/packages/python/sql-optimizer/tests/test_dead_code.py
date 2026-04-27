"""DeadCodeElimination — FALSE filters, LIMIT 0, TRUE filters, EmptyResult propagation."""

from __future__ import annotations

from sql_planner import (
    Distinct,
    EmptyResult,
    Filter,
    Join,
    JoinKind,
    Literal,
    LogicalPlan,
    Project,
    ProjectionItem,
    Scan,
    Sort,
    Union,
    Wildcard,
)
from sql_planner.plan import Limit, SortKey

from sql_optimizer import DeadCodeElimination


def dce(plan: LogicalPlan) -> LogicalPlan:
    return DeadCodeElimination()(plan)


class TestFalseFilter:
    def test_filter_false_becomes_empty(self) -> None:
        tree = Filter(input=Scan(table="t"), predicate=Literal(False))
        assert isinstance(dce(tree), EmptyResult)

    def test_filter_null_becomes_empty(self) -> None:
        tree = Filter(input=Scan(table="t"), predicate=Literal(None))
        assert isinstance(dce(tree), EmptyResult)


class TestTrueFilter:
    def test_filter_true_is_removed(self) -> None:
        inner = Scan(table="t")
        tree = Filter(input=inner, predicate=Literal(True))
        assert dce(tree) == inner


class TestLimitZero:
    def test_limit_zero_becomes_empty(self) -> None:
        tree = Limit(input=Scan(table="t"), count=0)
        assert isinstance(dce(tree), EmptyResult)


class TestEmptyPropagation:
    def test_project_empty(self) -> None:
        tree = Project(
            input=Filter(input=Scan(table="t"), predicate=Literal(False)),
            items=(ProjectionItem(expr=Wildcard(), alias=None),),
        )
        assert isinstance(dce(tree), EmptyResult)

    def test_sort_empty(self) -> None:
        tree = Sort(
            input=Filter(input=Scan(table="t"), predicate=Literal(False)),
            keys=(SortKey(expr=Wildcard()),),
        )
        assert isinstance(dce(tree), EmptyResult)

    def test_distinct_empty(self) -> None:
        tree = Distinct(
            input=Filter(input=Scan(table="t"), predicate=Literal(False))
        )
        assert isinstance(dce(tree), EmptyResult)

    def test_limit_empty(self) -> None:
        tree = Limit(
            input=Filter(input=Scan(table="t"), predicate=Literal(False)), count=10
        )
        assert isinstance(dce(tree), EmptyResult)


class TestJoinEmpty:
    def test_inner_join_left_empty(self) -> None:
        tree = Join(
            left=Filter(input=Scan(table="a"), predicate=Literal(False)),
            right=Scan(table="b"),
            kind=JoinKind.INNER,
            condition=None,
        )
        assert isinstance(dce(tree), EmptyResult)

    def test_cross_join_right_empty(self) -> None:
        tree = Join(
            left=Scan(table="a"),
            right=Filter(input=Scan(table="b"), predicate=Literal(False)),
            kind=JoinKind.CROSS,
            condition=None,
        )
        assert isinstance(dce(tree), EmptyResult)

    def test_left_join_empty_right_does_not_collapse(self) -> None:
        # LEFT JOIN empty preserves the left side.
        tree = Join(
            left=Scan(table="a"),
            right=Filter(input=Scan(table="b"), predicate=Literal(False)),
            kind=JoinKind.LEFT,
            condition=None,
        )
        out = dce(tree)
        assert isinstance(out, Join)


class TestUnionEmpty:
    def test_union_left_empty(self) -> None:
        tree = Union(
            left=Filter(input=Scan(table="a"), predicate=Literal(False)),
            right=Scan(table="b"),
        )
        assert dce(tree) == Scan(table="b")

    def test_union_right_empty(self) -> None:
        tree = Union(
            left=Scan(table="a"),
            right=Filter(input=Scan(table="b"), predicate=Literal(False)),
        )
        assert dce(tree) == Scan(table="a")


class TestIdempotent:
    def test_twice_same(self) -> None:
        tree = Filter(input=Scan(table="t"), predicate=Literal(False))
        once = dce(tree)
        twice = dce(once)
        assert once == twice
