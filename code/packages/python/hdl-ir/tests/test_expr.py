"""Tests for HIR expression nodes."""

import pytest

from hdl_ir.expr import (
    Attribute,
    BinaryOp,
    Concat,
    FunCall,
    Lit,
    NetRef,
    PortRef,
    Replication,
    Slice,
    SystemCall,
    Ternary,
    UnaryOp,
    VarRef,
    expr_from_dict,
)
from hdl_ir.provenance import Provenance, SourceLang, SourceLocation
from hdl_ir.types import TyLogic, TyVector


# ---- Atoms ----


def test_lit_int_round_trip():
    e = Lit(value=42, type=TyLogic())
    assert expr_from_dict(e.to_dict()) == e


def test_lit_with_provenance():
    prov = Provenance(SourceLang.VERILOG, SourceLocation("a.v", 5, 10))
    e = Lit(value=1, type=TyLogic(), provenance=prov)
    assert expr_from_dict(e.to_dict()) == e


def test_lit_vector_value_round_trip():
    e = Lit(value=(1, 0, 1, 0), type=TyVector(TyLogic(), 4))
    rt = expr_from_dict(e.to_dict())
    assert rt == e


def test_net_ref_round_trip():
    e = NetRef("clk")
    assert expr_from_dict(e.to_dict()) == e


def test_var_ref_round_trip():
    e = VarRef("counter")
    assert expr_from_dict(e.to_dict()) == e


def test_port_ref_round_trip():
    e = PortRef("a")
    assert expr_from_dict(e.to_dict()) == e


# ---- Composite ----


def test_slice_round_trip():
    e = Slice(base=PortRef("a"), msb=3, lsb=0)
    assert expr_from_dict(e.to_dict()) == e


def test_concat_round_trip():
    e = Concat((PortRef("cout"), PortRef("sum")))
    assert expr_from_dict(e.to_dict()) == e


def test_concat_empty_rejected():
    with pytest.raises(ValueError, match=">= 1 part"):
        Concat(())


def test_replication_round_trip():
    e = Replication(count=Lit(4, TyLogic()), body=PortRef("a"))
    assert expr_from_dict(e.to_dict()) == e


# ---- Operators ----


def test_unary_round_trip():
    e = UnaryOp("NOT", PortRef("a"))
    assert expr_from_dict(e.to_dict()) == e


def test_unary_unknown_op_rejected():
    with pytest.raises(ValueError, match="unknown unary"):
        UnaryOp("NOT_A_REAL_OP", PortRef("a"))


def test_binary_round_trip():
    e = BinaryOp("+", PortRef("a"), PortRef("b"))
    assert expr_from_dict(e.to_dict()) == e


def test_binary_unknown_op_rejected():
    with pytest.raises(ValueError, match="unknown binary"):
        BinaryOp("@@", PortRef("a"), PortRef("b"))


def test_ternary_round_trip():
    e = Ternary(
        cond=PortRef("sel"),
        then_expr=PortRef("a"),
        else_expr=PortRef("b"),
    )
    assert expr_from_dict(e.to_dict()) == e


# ---- Calls ----


def test_fun_call_round_trip():
    e = FunCall("max", (PortRef("a"), PortRef("b")))
    assert expr_from_dict(e.to_dict()) == e


def test_system_call_round_trip():
    e = SystemCall("$display", (Lit("hello", TyLogic()),))
    assert expr_from_dict(e.to_dict()) == e


def test_system_call_must_start_with_dollar():
    with pytest.raises(ValueError, match=r"\$"):
        SystemCall("display", ())


def test_attribute_round_trip():
    e = Attribute(base=NetRef("clk"), name="event")
    assert expr_from_dict(e.to_dict()) == e


def test_attribute_with_args_round_trip():
    e = Attribute(
        base=NetRef("data"), name="range", args=(Lit(0, TyLogic()), Lit(7, TyLogic()))
    )
    assert expr_from_dict(e.to_dict()) == e


# ---- Nested complex expression ----


def test_complex_expression_round_trip():
    # {cout, sum} = (a + b) + cin
    e = Concat(
        parts=(
            Slice(base=PortRef("cout"), msb=0, lsb=0),
            Slice(base=PortRef("sum"), msb=3, lsb=0),
        )
    )
    rt = expr_from_dict(e.to_dict())
    assert rt == e


def test_unknown_kind_rejected():
    with pytest.raises(ValueError, match="unknown expression"):
        expr_from_dict({"kind": "made_up_node"})
