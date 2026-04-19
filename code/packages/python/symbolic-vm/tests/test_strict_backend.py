"""Tests for :class:`StrictBackend`.

Strict mode is the "calculator" backend: every name must resolve,
every head must have a handler, and every arithmetic operation must
fully fold to a number.
"""

from __future__ import annotations

import pytest
from symbolic_ir import (
    ADD,
    ASSIGN,
    COS,
    DEFINE,
    DIV,
    EQUAL,
    LESS,
    LIST,
    MUL,
    NEG,
    POW,
    SIN,
    SUB,
    IRApply,
    IRFloat,
    IRInteger,
    IRRational,
    IRSymbol,
)

from symbolic_vm import VM, StrictBackend


@pytest.fixture
def vm() -> VM:
    return VM(StrictBackend())


# ---------------------------------------------------------------------------
# Literals pass through
# ---------------------------------------------------------------------------


def test_integer_literal(vm: VM) -> None:
    assert vm.eval(IRInteger(7)) == IRInteger(7)


def test_float_literal(vm: VM) -> None:
    assert vm.eval(IRFloat(3.14)) == IRFloat(3.14)


def test_rational_literal(vm: VM) -> None:
    assert vm.eval(IRRational(1, 3)) == IRRational(1, 3)


# ---------------------------------------------------------------------------
# Arithmetic
# ---------------------------------------------------------------------------


def test_add_integers(vm: VM) -> None:
    expr = IRApply(ADD, (IRInteger(2), IRInteger(3)))
    assert vm.eval(expr) == IRInteger(5)


def test_add_rationals_stays_exact(vm: VM) -> None:
    expr = IRApply(ADD, (IRRational(1, 2), IRRational(1, 3)))
    assert vm.eval(expr) == IRRational(5, 6)


def test_mul_float_contaminates(vm: VM) -> None:
    expr = IRApply(MUL, (IRInteger(2), IRFloat(0.5)))
    assert vm.eval(expr) == IRFloat(1.0)


def test_sub(vm: VM) -> None:
    expr = IRApply(SUB, (IRInteger(10), IRInteger(4)))
    assert vm.eval(expr) == IRInteger(6)


def test_div_stays_exact(vm: VM) -> None:
    expr = IRApply(DIV, (IRInteger(1), IRInteger(4)))
    assert vm.eval(expr) == IRRational(1, 4)


def test_div_by_zero_raises(vm: VM) -> None:
    with pytest.raises(ZeroDivisionError):
        vm.eval(IRApply(DIV, (IRInteger(1), IRInteger(0))))


def test_pow_integer(vm: VM) -> None:
    expr = IRApply(POW, (IRInteger(2), IRInteger(10)))
    assert vm.eval(expr) == IRInteger(1024)


def test_neg(vm: VM) -> None:
    assert vm.eval(IRApply(NEG, (IRInteger(5),))) == IRInteger(-5)


def test_nested_arithmetic(vm: VM) -> None:
    # (1 + 2) * (4 - 1) → 9
    expr = IRApply(
        MUL,
        (
            IRApply(ADD, (IRInteger(1), IRInteger(2))),
            IRApply(SUB, (IRInteger(4), IRInteger(1))),
        ),
    )
    assert vm.eval(expr) == IRInteger(9)


# ---------------------------------------------------------------------------
# Unresolved symbols raise
# ---------------------------------------------------------------------------


def test_unresolved_symbol_raises(vm: VM) -> None:
    with pytest.raises(NameError, match="undefined symbol: 'x'"):
        vm.eval(IRSymbol("x"))


def test_unknown_head_raises(vm: VM) -> None:
    with pytest.raises(NameError, match="no handler for head"):
        vm.eval(IRApply(IRSymbol("zaphod"), (IRInteger(1),)))


def test_add_with_symbol_raises(vm: VM) -> None:
    with pytest.raises(NameError):
        vm.eval(IRApply(ADD, (IRSymbol("x"), IRInteger(1))))


# ---------------------------------------------------------------------------
# Assignment
# ---------------------------------------------------------------------------


def test_assign_stores_value(vm: VM) -> None:
    vm.eval(IRApply(ASSIGN, (IRSymbol("a"), IRInteger(5))))
    assert vm.eval(IRSymbol("a")) == IRInteger(5)


def test_assign_then_use(vm: VM) -> None:
    vm.eval(IRApply(ASSIGN, (IRSymbol("a"), IRInteger(3))))
    vm.eval(IRApply(ASSIGN, (IRSymbol("b"), IRInteger(4))))
    expr = IRApply(ADD, (IRSymbol("a"), IRSymbol("b")))
    assert vm.eval(expr) == IRInteger(7)


def test_assign_rhs_is_evaluated(vm: VM) -> None:
    # a : 2 + 3 → a is bound to 5, not to Add(2, 3).
    rhs = IRApply(ADD, (IRInteger(2), IRInteger(3)))
    vm.eval(IRApply(ASSIGN, (IRSymbol("a"), rhs)))
    assert vm.eval(IRSymbol("a")) == IRInteger(5)


# ---------------------------------------------------------------------------
# Function definition and application
# ---------------------------------------------------------------------------


def test_define_and_call(vm: VM) -> None:
    # square(x) := x^2
    defn = IRApply(
        DEFINE,
        (
            IRSymbol("square"),
            IRApply(LIST, (IRSymbol("x"),)),
            IRApply(POW, (IRSymbol("x"), IRInteger(2))),
        ),
    )
    vm.eval(defn)
    # square(7)
    call = IRApply(IRSymbol("square"), (IRInteger(7),))
    assert vm.eval(call) == IRInteger(49)


def test_define_multi_arg(vm: VM) -> None:
    defn = IRApply(
        DEFINE,
        (
            IRSymbol("plus"),
            IRApply(LIST, (IRSymbol("x"), IRSymbol("y"))),
            IRApply(ADD, (IRSymbol("x"), IRSymbol("y"))),
        ),
    )
    vm.eval(defn)
    assert vm.eval(
        IRApply(IRSymbol("plus"), (IRInteger(10), IRInteger(20)))
    ) == IRInteger(30)


def test_user_function_arity_mismatch(vm: VM) -> None:
    defn = IRApply(
        DEFINE,
        (
            IRSymbol("square"),
            IRApply(LIST, (IRSymbol("x"),)),
            IRApply(POW, (IRSymbol("x"), IRInteger(2))),
        ),
    )
    vm.eval(defn)
    with pytest.raises(TypeError, match="arity mismatch"):
        vm.eval(IRApply(IRSymbol("square"), (IRInteger(1), IRInteger(2))))


# ---------------------------------------------------------------------------
# Elementary functions
# ---------------------------------------------------------------------------


def test_sin_zero_exact(vm: VM) -> None:
    assert vm.eval(IRApply(SIN, (IRInteger(0),))) == IRInteger(0)


def test_cos_zero_exact(vm: VM) -> None:
    assert vm.eval(IRApply(COS, (IRInteger(0),))) == IRInteger(1)


# ---------------------------------------------------------------------------
# Comparisons
# ---------------------------------------------------------------------------


def test_equal_true(vm: VM) -> None:
    assert vm.eval(IRApply(EQUAL, (IRInteger(3), IRInteger(3)))) == IRSymbol("True")


def test_less_true(vm: VM) -> None:
    assert vm.eval(IRApply(LESS, (IRInteger(2), IRInteger(5)))) == IRSymbol("True")


def test_less_false(vm: VM) -> None:
    assert vm.eval(IRApply(LESS, (IRInteger(5), IRInteger(2)))) == IRSymbol("False")


# ---------------------------------------------------------------------------
# Program evaluation
# ---------------------------------------------------------------------------


def test_eval_program_returns_last(vm: VM) -> None:
    stmts = [
        IRApply(ASSIGN, (IRSymbol("a"), IRInteger(10))),
        IRApply(ASSIGN, (IRSymbol("b"), IRInteger(20))),
        IRApply(ADD, (IRSymbol("a"), IRSymbol("b"))),
    ]
    assert vm.eval_program(stmts) == IRInteger(30)


def test_eval_program_empty(vm: VM) -> None:
    assert vm.eval_program([]) is None
