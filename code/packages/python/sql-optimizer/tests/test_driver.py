"""Driver — optimize() runs full pipeline; optimize_with_passes() is configurable."""

from __future__ import annotations

from sql_planner import (
    BinaryExpr,
    BinaryOp,
    Column,
    EmptyResult,
    Filter,
    Literal,
    Project,
    ProjectionItem,
    Scan,
)

from sql_optimizer import (
    ConstantFolding,
    DeadCodeElimination,
    LimitPushdown,
    Pass,
    PredicatePushdown,
    ProjectionPruning,
    default_passes,
    optimize,
    optimize_with_passes,
)


class TestDefaultPasses:
    def test_five_passes_in_order(self) -> None:
        passes = default_passes()
        names = [p.name for p in passes]
        assert names == [
            "ConstantFolding",
            "PredicatePushdown",
            "ProjectionPruning",
            "DeadCodeElimination",
            "LimitPushdown",
        ]


class TestFullPipeline:
    def test_constant_false_filter_becomes_empty(self) -> None:
        # ConstantFolding → Filter(FALSE); DeadCodeElimination → EmptyResult.
        tree = Project(
            input=Filter(
                input=Scan(table="t"),
                predicate=BinaryExpr(
                    op=BinaryOp.EQ, left=Literal(1), right=Literal(2)
                ),
            ),
            items=(ProjectionItem(expr=Column("t", "x"), alias="x"),),
        )
        assert isinstance(optimize(tree), EmptyResult)

    def test_single_pass_isolation(self) -> None:
        tree = Filter(
            input=Scan(table="t"),
            predicate=BinaryExpr(op=BinaryOp.ADD, left=Literal(1), right=Literal(2)),
        )
        # Only ConstantFolding — no dead-code removal, no scan annotation.
        out = optimize_with_passes(tree, [ConstantFolding()])
        assert isinstance(out, Filter)
        assert out.predicate == Literal(3)


class TestProtocolConformance:
    def test_passes_are_pass_instances(self) -> None:
        for p in (
            ConstantFolding(),
            PredicatePushdown(),
            ProjectionPruning(),
            DeadCodeElimination(),
            LimitPushdown(),
        ):
            assert isinstance(p, Pass)
