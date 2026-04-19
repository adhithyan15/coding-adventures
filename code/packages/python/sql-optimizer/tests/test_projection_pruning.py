"""ProjectionPruning — required-column annotation on Scan nodes."""

from __future__ import annotations

from sql_planner import (
    BinaryExpr,
    BinaryOp,
    Column,
    Filter,
    Join,
    JoinKind,
    Literal,
    LogicalPlan,
    Project,
    ProjectionItem,
    Scan,
    Wildcard,
)

from sql_optimizer import ProjectionPruning


def pr(plan: LogicalPlan) -> LogicalPlan:
    return ProjectionPruning()(plan)


class TestBasic:
    def test_project_annotates_scan(self) -> None:
        tree = Project(
            input=Scan(table="t", alias="t"),
            items=(ProjectionItem(expr=Column("t", "name"), alias="name"),),
        )
        out = pr(tree)
        assert isinstance(out, Project)
        assert isinstance(out.input, Scan)
        assert out.input.required_columns == ("name",)


class TestFilterAddsToRequired:
    def test_filter_columns_carry_down(self) -> None:
        tree = Project(
            input=Filter(
                input=Scan(table="t", alias="t"),
                predicate=BinaryExpr(
                    op=BinaryOp.GT, left=Column("t", "age"), right=Literal(18)
                ),
            ),
            items=(ProjectionItem(expr=Column("t", "name"), alias="name"),),
        )
        out = pr(tree)
        assert isinstance(out, Project)
        scan = out.input.input
        assert set(scan.required_columns) == {"name", "age"}


class TestJoinDistributesRequirements:
    def test_join_sides_get_their_columns(self) -> None:
        tree = Project(
            input=Join(
                left=Scan(table="a", alias="a"),
                right=Scan(table="b", alias="b"),
                kind=JoinKind.INNER,
                condition=BinaryExpr(
                    op=BinaryOp.EQ, left=Column("a", "k"), right=Column("b", "k")
                ),
            ),
            items=(
                ProjectionItem(expr=Column("a", "name"), alias="name"),
            ),
        )
        out = pr(tree)
        left = out.input.left
        right = out.input.right
        # Left needs ``name`` (from projection) + ``k`` (from join cond).
        assert set(left.required_columns) == {"name", "k"}
        # Right only needs ``k`` (from join cond — parent doesn't need any).
        assert set(right.required_columns) == {"k"}


class TestWildcardPreventsPruning:
    def test_wildcard_leaves_scan_unannotated(self) -> None:
        tree = Project(
            input=Scan(table="t"),
            items=(ProjectionItem(expr=Wildcard(), alias=None),),
        )
        out = pr(tree)
        assert isinstance(out.input, Scan)
        # required_columns should be an empty tuple (all columns) or None.
        # Either way, not restricting.
        assert out.input.required_columns in (None, ())


class TestIdempotent:
    def test_twice_same(self) -> None:
        tree = Project(
            input=Scan(table="t", alias="t"),
            items=(ProjectionItem(expr=Column("t", "x"), alias="x"),),
        )
        once = pr(tree)
        twice = pr(once)
        assert once == twice
