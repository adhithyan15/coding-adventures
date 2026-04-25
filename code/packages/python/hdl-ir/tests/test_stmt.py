"""Tests for HIR statement nodes."""

import pytest

from hdl_ir.expr import BinaryOp, Lit, NetRef, PortRef
from hdl_ir.stmt import (
    AssertStmt,
    BlockingAssign,
    CaseItem,
    CaseStmt,
    DelayStmt,
    DisableStmt,
    Event,
    EventStmt,
    ExprStmt,
    ForeverStmt,
    ForStmt,
    IfStmt,
    NonblockingAssign,
    NullStmt,
    RepeatStmt,
    ReportStmt,
    ReturnStmt,
    WaitStmt,
    WhileStmt,
    stmt_from_dict,
)
from hdl_ir.types import TyLogic


def test_blocking_round_trip():
    s = BlockingAssign(target=NetRef("a"), rhs=Lit(1, TyLogic()))
    assert stmt_from_dict(s.to_dict()) == s


def test_blocking_with_delay_round_trip():
    s = BlockingAssign(
        target=NetRef("a"), rhs=Lit(1, TyLogic()), delay=Lit(10, TyLogic())
    )
    assert stmt_from_dict(s.to_dict()) == s


def test_nonblocking_round_trip():
    s = NonblockingAssign(target=NetRef("q"), rhs=PortRef("d"))
    assert stmt_from_dict(s.to_dict()) == s


def test_if_no_else_round_trip():
    s = IfStmt(
        cond=BinaryOp("==", NetRef("reset"), Lit(1, TyLogic())),
        then_branch=(NonblockingAssign(NetRef("count"), Lit(0, TyLogic())),),
    )
    assert stmt_from_dict(s.to_dict()) == s


def test_if_with_else_round_trip():
    s = IfStmt(
        cond=BinaryOp("==", NetRef("reset"), Lit(1, TyLogic())),
        then_branch=(NonblockingAssign(NetRef("count"), Lit(0, TyLogic())),),
        else_branch=(
            NonblockingAssign(
                NetRef("count"),
                BinaryOp("+", NetRef("count"), Lit(1, TyLogic())),
            ),
        ),
    )
    assert stmt_from_dict(s.to_dict()) == s


def test_case_round_trip():
    s = CaseStmt(
        expr=NetRef("state"),
        items=(
            CaseItem(
                choices=(Lit(0, TyLogic()),),
                body=(NonblockingAssign(NetRef("state"), Lit(1, TyLogic())),),
            ),
            CaseItem(
                choices=(Lit(1, TyLogic()),),
                body=(NonblockingAssign(NetRef("state"), Lit(0, TyLogic())),),
            ),
        ),
        default=(NullStmt(),),
    )
    assert stmt_from_dict(s.to_dict()) == s


def test_case_invalid_kind_rejected():
    with pytest.raises(ValueError, match="unknown case kind"):
        CaseStmt(expr=NetRef("x"), items=(), kind_="not_a_kind")


def test_for_round_trip():
    s = ForStmt(
        init=BlockingAssign(NetRef("i"), Lit(0, TyLogic())),
        cond=BinaryOp("<", NetRef("i"), Lit(8, TyLogic())),
        step=BlockingAssign(NetRef("i"), BinaryOp("+", NetRef("i"), Lit(1, TyLogic()))),
        body=(NullStmt(),),
    )
    assert stmt_from_dict(s.to_dict()) == s


def test_while_round_trip():
    s = WhileStmt(cond=NetRef("ready"), body=(NullStmt(),))
    assert stmt_from_dict(s.to_dict()) == s


def test_repeat_round_trip():
    s = RepeatStmt(count=Lit(5, TyLogic()), body=(NullStmt(),))
    assert stmt_from_dict(s.to_dict()) == s


def test_forever_round_trip():
    s = ForeverStmt(body=(NullStmt(),))
    assert stmt_from_dict(s.to_dict()) == s


def test_wait_no_args_round_trip():
    s = WaitStmt()
    assert stmt_from_dict(s.to_dict()) == s


def test_wait_with_on_until_for_round_trip():
    s = WaitStmt(
        on=(NetRef("a"), NetRef("b")),
        until=NetRef("ready"),
        for_=Lit(100, TyLogic()),
    )
    assert stmt_from_dict(s.to_dict()) == s


def test_delay_round_trip():
    s = DelayStmt(amount=Lit(10, TyLogic()), body=(NullStmt(),))
    assert stmt_from_dict(s.to_dict()) == s


def test_event_round_trip():
    s = EventStmt(
        events=(Event(edge="posedge", expr=NetRef("clk")),),
        body=(NonblockingAssign(NetRef("q"), NetRef("d")),),
    )
    assert stmt_from_dict(s.to_dict()) == s


def test_event_unknown_edge_rejected():
    with pytest.raises(ValueError, match="unknown edge"):
        Event(edge="nope", expr=NetRef("clk"))


def test_assert_round_trip():
    s = AssertStmt(
        cond=BinaryOp("==", NetRef("a"), Lit(1, TyLogic())),
        message=Lit("a should be 1", TyLogic()),
        severity="warning",
    )
    assert stmt_from_dict(s.to_dict()) == s


def test_assert_unknown_severity_rejected():
    with pytest.raises(ValueError, match="severity"):
        AssertStmt(cond=NetRef("x"), severity="catastrophic")


def test_report_round_trip():
    s = ReportStmt(message=Lit("hi", TyLogic()))
    assert stmt_from_dict(s.to_dict()) == s


def test_disable_round_trip():
    s = DisableStmt(target="my_block")
    assert stmt_from_dict(s.to_dict()) == s


def test_return_round_trip():
    s = ReturnStmt(value=Lit(42, TyLogic()))
    assert stmt_from_dict(s.to_dict()) == s


def test_return_no_value_round_trip():
    s = ReturnStmt()
    assert stmt_from_dict(s.to_dict()) == s


def test_null_round_trip():
    s = NullStmt()
    assert stmt_from_dict(s.to_dict()) == s


def test_expr_stmt_round_trip():
    s = ExprStmt(expr=NetRef("foo"))
    assert stmt_from_dict(s.to_dict()) == s


def test_unknown_kind_rejected():
    with pytest.raises(ValueError, match="unknown statement"):
        stmt_from_dict({"kind": "no_such_stmt"})
