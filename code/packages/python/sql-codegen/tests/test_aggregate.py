"""Aggregate — InitAgg / UpdateAgg / FinalizeAgg, SaveGroupKey, HAVING."""

from __future__ import annotations

from sql_planner import (
    AggFunc,
    Aggregate,
    AggregateItem,
    BinaryExpr,
    Column,
    FuncArg,
    Having,
    Literal,
    Scan,
)
from sql_planner import (
    BinaryOp as AstOp,
)
from sql_planner.expr import AggregateExpr

from sql_codegen import (
    BeginRow,
    EmitColumn,
    EmitRow,
    FinalizeAgg,
    InitAgg,
    IrAggFunc,
    JumpIfFalse,
    LoadGroupKey,
    SaveGroupKey,
    compile,
)


def test_aggregate_emits_init_update_finalize() -> None:
    plan = Aggregate(
        input=Scan(table="t", alias="t"),
        group_by=(Column("t", "region"),),
        aggregates=(
            AggregateItem(
                func=AggFunc.COUNT,
                arg=FuncArg(star=True),
                alias="cnt",
            ),
        ),
    )
    prog = compile(plan)
    kinds = {type(i).__name__ for i in prog.instructions}
    assert "InitAgg" in kinds
    assert "UpdateAgg" in kinds
    assert "FinalizeAgg" in kinds
    assert "SaveGroupKey" in kinds


def test_save_group_key_count_matches_group_by() -> None:
    plan = Aggregate(
        input=Scan(table="t", alias="t"),
        group_by=(Column("t", "a"), Column("t", "b")),
        aggregates=(),
    )
    prog = compile(plan)
    keys = [i for i in prog.instructions if isinstance(i, SaveGroupKey)]
    assert keys[0].n == 2


def test_init_agg_func_mapped() -> None:
    plan = Aggregate(
        input=Scan(table="t", alias="t"),
        group_by=(),
        aggregates=(
            AggregateItem(
                func=AggFunc.SUM,
                arg=FuncArg(value=Column("t", "x")),
                alias="s",
            ),
        ),
    )
    prog = compile(plan)
    inits = [i for i in prog.instructions if isinstance(i, InitAgg)]
    assert inits[0].func is IrAggFunc.SUM


def test_having_inserts_jumpiffalse() -> None:
    agg = Aggregate(
        input=Scan(table="t", alias="t"),
        group_by=(Column("t", "region"),),
        aggregates=(
            AggregateItem(
                func=AggFunc.COUNT,
                arg=FuncArg(star=True),
                alias="cnt",
            ),
        ),
    )
    plan = Having(
        input=agg,
        predicate=BinaryExpr(
            op=AstOp.GT,
            left=AggregateExpr(func=AggFunc.COUNT, arg=None),
            right=Literal(2),
        ),
    )
    prog = compile(plan)
    jumps = [i for i in prog.instructions if isinstance(i, JumpIfFalse)]
    assert len(jumps) >= 1


def test_aggregate_emits_one_row_per_group() -> None:
    plan = Aggregate(
        input=Scan(table="t", alias="t"),
        group_by=(Column("t", "r"),),
        aggregates=(
            AggregateItem(
                func=AggFunc.COUNT,
                arg=FuncArg(star=True),
                alias="cnt",
            ),
        ),
    )
    prog = compile(plan)
    # BeginRow + EmitColumn×2 (r + cnt) + EmitRow forms the per-group emit block.
    assert any(isinstance(i, BeginRow) for i in prog.instructions)
    assert any(isinstance(i, EmitRow) for i in prog.instructions)
    assert any(isinstance(i, LoadGroupKey) for i in prog.instructions)
    finalized = [i for i in prog.instructions if isinstance(i, FinalizeAgg)]
    assert len(finalized) >= 1
    # ``cnt`` column emitted.
    emits = [i for i in prog.instructions if isinstance(i, EmitColumn)]
    assert any(e.name == "cnt" for e in emits)


def test_count_star_pushes_sentinel() -> None:
    from sql_codegen import LoadConst

    plan = Aggregate(
        input=Scan(table="t", alias="t"),
        group_by=(),
        aggregates=(
            AggregateItem(
                func=AggFunc.COUNT,
                arg=FuncArg(star=True),
                alias="c",
            ),
        ),
    )
    prog = compile(plan)
    # A LoadConst(1) feeds UpdateAgg for COUNT(*).
    assert LoadConst(value=1) in list(prog.instructions)
