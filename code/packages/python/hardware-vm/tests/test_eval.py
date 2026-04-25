"""Tests for the HIR expression evaluator."""

import pytest

from hardware_vm.eval import evaluate, referenced_signals
from hdl_ir import (
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
    TyLogic,
    TyVector,
    UnaryOp,
    VarRef,
)


def lookup(name: str) -> int:
    return {"a": 5, "b": 3, "x": 0xFF, "y": 0x0F, "z": 0}.get(name, 0)


# ---- Literals ----


def test_lit_int():
    assert evaluate(Lit(value=42, type=TyLogic()), lookup) == 42


def test_lit_bool():
    assert evaluate(Lit(value=True, type=TyLogic()), lookup) == 1


def test_lit_tuple_packs_msb_first():
    # (1, 0, 1, 0) -> binary 1010 -> 10
    assert evaluate(Lit(value=(1, 0, 1, 0), type=TyVector(TyLogic(), 4)), lookup) == 10


def test_lit_string_numeric():
    assert evaluate(Lit(value="42", type=TyLogic()), lookup) == 42


def test_lit_string_non_numeric():
    assert evaluate(Lit(value="hi", type=TyLogic()), lookup) == 0


# ---- Refs ----


def test_net_ref():
    assert evaluate(NetRef("a"), lookup) == 5


def test_port_ref():
    assert evaluate(PortRef("b"), lookup) == 3


def test_var_ref():
    assert evaluate(VarRef("x"), lookup) == 0xFF


# ---- Binary ops ----


@pytest.mark.parametrize(
    "op,a,b,expected",
    [
        ("+", 5, 3, 8),
        ("-", 5, 3, 2),
        ("*", 5, 3, 15),
        ("/", 10, 3, 3),
        ("/", 10, 0, 0),  # divide-by-zero returns 0
        ("%", 10, 3, 1),
        ("%", 10, 0, 0),
        ("**", 2, 8, 256),
        ("&", 0xF0, 0x0F, 0x00),
        ("|", 0xF0, 0x0F, 0xFF),
        ("^", 0xFF, 0x0F, 0xF0),
        ("AND", 0xF0, 0x0F, 0x00),
        ("OR", 0xF0, 0x0F, 0xFF),
        ("XOR", 0xFF, 0x0F, 0xF0),
        ("<<", 1, 4, 16),
        (">>", 16, 4, 1),
        ("==", 5, 5, 1),
        ("==", 5, 3, 0),
        ("!=", 5, 3, 1),
        ("<", 3, 5, 1),
        ("<", 5, 3, 0),
        (">", 5, 3, 1),
        ("<=", 3, 3, 1),
        (">=", 5, 5, 1),
        ("&&", 1, 1, 1),
        ("&&", 1, 0, 0),
        ("||", 1, 0, 1),
        ("||", 0, 0, 0),
    ],
)
def test_binary_op(op, a, b, expected):
    expr = BinaryOp(op, Lit(a, TyLogic()), Lit(b, TyLogic()))
    assert evaluate(expr, lookup) == expected


def test_binary_unknown_op():
    expr = BinaryOp("+", Lit(1, TyLogic()), Lit(2, TyLogic()))
    # Manually break the op
    object.__setattr__(expr, "op", "@@@")
    with pytest.raises(ValueError, match="unknown binary"):
        evaluate(expr, lookup)


# ---- Unary ops ----


def test_unary_neg():
    assert evaluate(UnaryOp("NEG", Lit(5, TyLogic())), lookup) == -5


def test_unary_logic_not_zero():
    assert evaluate(UnaryOp("LOGIC_NOT", Lit(0, TyLogic())), lookup) == 1


def test_unary_logic_not_nonzero():
    assert evaluate(UnaryOp("LOGIC_NOT", Lit(5, TyLogic())), lookup) == 0


def test_unary_or_red():
    assert evaluate(UnaryOp("OR_RED", Lit(0, TyLogic())), lookup) == 0
    assert evaluate(UnaryOp("OR_RED", Lit(5, TyLogic())), lookup) == 1


def test_unary_xor_red():
    assert evaluate(UnaryOp("XOR_RED", Lit(0b1011, TyLogic())), lookup) == 1
    assert evaluate(UnaryOp("XOR_RED", Lit(0b1100, TyLogic())), lookup) == 0


# ---- Slice ----


def test_slice_low_high():
    expr = Slice(base=NetRef("x"), msb=3, lsb=0)
    assert evaluate(expr, lookup) == 0xF


def test_slice_high_only():
    expr = Slice(base=NetRef("x"), msb=7, lsb=4)
    assert evaluate(expr, lookup) == 0xF


def test_slice_inverted():
    expr = Slice(base=NetRef("x"), msb=0, lsb=3)
    assert evaluate(expr, lookup) == 0xF


# ---- Concat ----


def test_concat_pure_lits():
    expr = Concat(parts=(Lit(1, TyLogic()), Lit(0, TyLogic())))
    assert evaluate(expr, lookup) == 0b10


def test_concat_with_vector_lit():
    expr = Concat(
        parts=(Lit(1, TyLogic()), Lit(value=(1, 0, 1, 0), type=TyVector(TyLogic(), 4)))
    )
    # 1 ++ 1010 = 11010 = 26
    assert evaluate(expr, lookup) == 26


# ---- Replication ----


def test_replication():
    expr = Replication(count=Lit(3, TyLogic()), body=Lit(1, TyLogic()))
    assert evaluate(expr, lookup) == 0b111


# ---- Ternary ----


def test_ternary_then():
    expr = Ternary(
        cond=Lit(1, TyLogic()), then_expr=Lit(42, TyLogic()), else_expr=Lit(0, TyLogic())
    )
    assert evaluate(expr, lookup) == 42


def test_ternary_else():
    expr = Ternary(
        cond=Lit(0, TyLogic()), then_expr=Lit(42, TyLogic()), else_expr=Lit(99, TyLogic())
    )
    assert evaluate(expr, lookup) == 99


# ---- Calls (no-op in v0.1.0) ----


def test_funcall_returns_zero():
    assert evaluate(FunCall("max", ()), lookup) == 0


def test_systemcall_returns_zero():
    assert evaluate(SystemCall("$display", ()), lookup) == 0


def test_attribute_returns_zero():
    assert evaluate(Attribute(base=NetRef("a"), name="event"), lookup) == 0


# ---- referenced_signals ----


def test_referenced_lit():
    assert referenced_signals(Lit(0, TyLogic())) == set()


def test_referenced_net_ref():
    assert referenced_signals(NetRef("clk")) == {"clk"}


def test_referenced_binary():
    expr = BinaryOp("+", PortRef("a"), NetRef("b"))
    assert referenced_signals(expr) == {"a", "b"}


def test_referenced_concat():
    expr = Concat((NetRef("a"), PortRef("b"), VarRef("c")))
    assert referenced_signals(expr) == {"a", "b", "c"}


def test_referenced_ternary():
    expr = Ternary(NetRef("s"), NetRef("a"), NetRef("b"))
    assert referenced_signals(expr) == {"s", "a", "b"}


def test_referenced_unary():
    expr = UnaryOp("NOT", NetRef("a"))
    assert referenced_signals(expr) == {"a"}


def test_referenced_slice():
    expr = Slice(base=NetRef("data"), msb=7, lsb=0)
    assert referenced_signals(expr) == {"data"}


def test_referenced_replication():
    expr = Replication(count=NetRef("n"), body=NetRef("v"))
    assert referenced_signals(expr) == {"n", "v"}


def test_referenced_attribute():
    expr = Attribute(base=NetRef("clk"), name="event")
    assert referenced_signals(expr) == {"clk"}


def test_referenced_funcall():
    expr = FunCall("max", (NetRef("a"), PortRef("b")))
    assert referenced_signals(expr) == {"a", "b"}
