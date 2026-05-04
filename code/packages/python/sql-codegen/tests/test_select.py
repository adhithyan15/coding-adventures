"""SELECT shape — Project, Filter, Scan, Join, Sort, Limit, Distinct."""

from __future__ import annotations

from sql_planner import (
    BinaryExpr,
    Column,
    Distinct,
    Filter,
    Join,
    JoinKind,
    Literal,
    Project,
    ProjectionItem,
    Scan,
    Sort,
    Wildcard,
)
from sql_planner import (
    BinaryOp as AstOp,
)
from sql_planner.plan import Limit, SortKey

from sql_codegen import (
    CloseScan,
    Direction,
    DistinctResult,
    JoinBeginRow,
    JoinIfMatched,
    JoinSetMatched,
    JumpIfFalse,
    LimitResult,
    OpenScan,
    ScanAllColumns,
    SetResultSchema,
    SortResult,
    compile,
)


class TestProject:
    def test_project_emits_row_buffer_sequence(self) -> None:
        plan = Project(
            input=Scan(table="t", alias="t"),
            items=(ProjectionItem(expr=Column("t", "x"), alias="x"),),
        )
        prog = compile(plan)
        kinds = [type(i).__name__ for i in prog.instructions]
        assert "BeginRow" in kinds
        assert "EmitColumn" in kinds
        assert "EmitRow" in kinds

    def test_wildcard_uses_scan_all_columns(self) -> None:
        plan = Project(
            input=Scan(table="t", alias="t"),
            items=(ProjectionItem(expr=Wildcard(), alias=None),),
        )
        prog = compile(plan)
        assert any(isinstance(i, ScanAllColumns) for i in prog.instructions)


class TestFilter:
    def test_filter_wraps_body_in_jumpiffalse(self) -> None:
        plan = Project(
            input=Filter(
                input=Scan(table="t", alias="t"),
                predicate=BinaryExpr(
                    op=AstOp.GT, left=Column("t", "x"), right=Literal(18)
                ),
            ),
            items=(ProjectionItem(expr=Column("t", "x"), alias="x"),),
        )
        prog = compile(plan)
        jumps = [i for i in prog.instructions if isinstance(i, JumpIfFalse)]
        assert len(jumps) >= 1
        # The skip target resolves to an instruction index.
        for j in jumps:
            assert j.label in prog.labels


class TestJoin:
    def test_inner_join_is_nested_loops(self) -> None:
        plan = Project(
            input=Join(
                left=Scan(table="a", alias="a"),
                right=Scan(table="b", alias="b"),
                kind=JoinKind.INNER,
                condition=BinaryExpr(
                    op=AstOp.EQ, left=Column("a", "k"), right=Column("b", "k")
                ),
            ),
            items=(ProjectionItem(expr=Column("a", "k"), alias="k"),),
        )
        prog = compile(plan)
        opens = [i for i in prog.instructions if isinstance(i, OpenScan)]
        assert len(opens) == 2
        # Distinct cursor IDs.
        assert opens[0].cursor_id != opens[1].cursor_id
        # Two CloseScans, one per cursor.
        closes = [i for i in prog.instructions if isinstance(i, CloseScan)]
        assert len(closes) == 2

    def test_cross_join_has_no_filter(self) -> None:
        plan = Project(
            input=Join(
                left=Scan(table="a", alias="a"),
                right=Scan(table="b", alias="b"),
                kind=JoinKind.CROSS,
                condition=None,
            ),
            items=(ProjectionItem(expr=Column("a", "x"), alias="x"),),
        )
        prog = compile(plan)
        # CROSS has no predicate, so there should be no extra JumpIfFalse
        # beyond what Filter / Having would add (none here).
        jumps = [i for i in prog.instructions if isinstance(i, JumpIfFalse)]
        assert len(jumps) == 0

    def test_left_join_compiles(self) -> None:
        # LEFT JOIN must compile without raising and emit the outer-join
        # match-tracking instructions: JoinBeginRow, JoinSetMatched,
        # JoinIfMatched.  Exactly one of each (two-table join).
        plan = Project(
            input=Join(
                left=Scan(table="a", alias="a"),
                right=Scan(table="b", alias="b"),
                kind=JoinKind.LEFT,
                condition=BinaryExpr(
                    op=AstOp.EQ, left=Column("a", "k"), right=Column("b", "k")
                ),
            ),
            items=(ProjectionItem(expr=Column("a", "k"), alias="k"),),
        )
        prog = compile(plan)
        assert any(isinstance(i, JoinBeginRow) for i in prog.instructions)
        assert any(isinstance(i, JoinSetMatched) for i in prog.instructions)
        matched = [i for i in prog.instructions if isinstance(i, JoinIfMatched)]
        assert len(matched) == 1
        # The matched label must resolve to a valid instruction index.
        assert matched[0].label in prog.labels
        # Still two OpenScan + two CloseScan (same structural shape as INNER).
        opens = [i for i in prog.instructions if isinstance(i, OpenScan)]
        closes = [i for i in prog.instructions if isinstance(i, CloseScan)]
        assert len(opens) == 2
        assert len(closes) == 2

    def test_right_join_raises(self) -> None:
        from sql_codegen.errors import UnsupportedNode

        plan = Project(
            input=Join(
                left=Scan(table="a", alias="a"),
                right=Scan(table="b", alias="b"),
                kind=JoinKind.RIGHT,
                condition=BinaryExpr(
                    op=AstOp.EQ, left=Column("a", "k"), right=Column("b", "k")
                ),
            ),
            items=(ProjectionItem(expr=Column("a", "k"), alias="k"),),
        )
        try:
            compile(plan)
        except UnsupportedNode:
            pass
        else:
            raise AssertionError("expected UnsupportedNode for RIGHT JOIN")


class TestPostProcessing:
    def test_sort_emits_sort_result(self) -> None:
        plan = Sort(
            input=Project(
                input=Scan(table="t", alias="t"),
                items=(ProjectionItem(expr=Column("t", "x"), alias="x"),),
            ),
            keys=(SortKey(expr=Column("t", "x"), descending=True),),
        )
        prog = compile(plan)
        sorts = [i for i in prog.instructions if isinstance(i, SortResult)]
        assert len(sorts) == 1
        assert sorts[0].keys[0].direction is Direction.DESC

    def test_limit_emits_limit_result(self) -> None:
        plan = Limit(
            input=Project(
                input=Scan(table="t", alias="t"),
                items=(ProjectionItem(expr=Column("t", "x"), alias="x"),),
            ),
            count=10,
            offset=5,
        )
        prog = compile(plan)
        limits = [i for i in prog.instructions if isinstance(i, LimitResult)]
        assert limits[0].count == 10
        assert limits[0].offset == 5

    def test_distinct_emits_distinct_result(self) -> None:
        plan = Distinct(
            input=Project(
                input=Scan(table="t", alias="t"),
                items=(ProjectionItem(expr=Column("t", "x"), alias="x"),),
            ),
        )
        prog = compile(plan)
        assert any(isinstance(i, DistinctResult) for i in prog.instructions)

    def test_sort_then_limit_runs_sort_first(self) -> None:
        plan = Limit(
            input=Sort(
                input=Project(
                    input=Scan(table="t", alias="t"),
                    items=(ProjectionItem(expr=Column("t", "x"), alias="x"),),
                ),
                keys=(SortKey(expr=Column("t", "x")),),
            ),
            count=3,
        )
        prog = compile(plan)
        sort_idx = next(
            i for i, ins in enumerate(prog.instructions) if isinstance(ins, SortResult)
        )
        limit_idx = next(
            i for i, ins in enumerate(prog.instructions) if isinstance(ins, LimitResult)
        )
        assert sort_idx < limit_idx


class TestSchema:
    def test_schema_uses_alias_when_present(self) -> None:
        plan = Project(
            input=Scan(table="t", alias="t"),
            items=(
                ProjectionItem(expr=Column("t", "x"), alias="renamed"),
                ProjectionItem(expr=Column("t", "y"), alias=None),
            ),
        )
        prog = compile(plan)
        assert prog.result_schema == ("renamed", "y")
        schemas = [i for i in prog.instructions if isinstance(i, SetResultSchema)]
        assert schemas[0].columns == ("renamed", "y")
