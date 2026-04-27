"""Tests for :class:`SymbolicBackend`.

Symbolic mode leaves unknown symbols alone, applies algebraic identities,
and implements the standard calculus rules for ``D``.
"""

from __future__ import annotations

import pytest
from symbolic_ir import (
    ADD,
    COS,
    DIV,
    EXP,
    LOG,
    MUL,
    NEG,
    POW,
    SIN,
    SUB,
    D,
    IRApply,
    IRInteger,
    IRSymbol,
)

from symbolic_vm import VM, SymbolicBackend


@pytest.fixture
def vm() -> VM:
    return VM(SymbolicBackend())


X = IRSymbol("x")
Y = IRSymbol("y")


# ---------------------------------------------------------------------------
# Free variables pass through
# ---------------------------------------------------------------------------


def test_free_symbol_passes_through(vm: VM) -> None:
    assert vm.eval(X) == X


def test_unknown_head_passes_through(vm: VM) -> None:
    expr = IRApply(IRSymbol("mysteryfn"), (IRInteger(1),))
    assert vm.eval(expr) == expr


# ---------------------------------------------------------------------------
# Identity and zero laws
# ---------------------------------------------------------------------------


def test_add_zero_right_identity(vm: VM) -> None:
    assert vm.eval(IRApply(ADD, (X, IRInteger(0)))) == X


def test_add_zero_left_identity(vm: VM) -> None:
    assert vm.eval(IRApply(ADD, (IRInteger(0), X))) == X


def test_mul_zero_annihilates(vm: VM) -> None:
    assert vm.eval(IRApply(MUL, (X, IRInteger(0)))) == IRInteger(0)
    assert vm.eval(IRApply(MUL, (IRInteger(0), X))) == IRInteger(0)


def test_mul_one_identity(vm: VM) -> None:
    assert vm.eval(IRApply(MUL, (X, IRInteger(1)))) == X
    assert vm.eval(IRApply(MUL, (IRInteger(1), X))) == X


def test_pow_zero_exponent(vm: VM) -> None:
    assert vm.eval(IRApply(POW, (X, IRInteger(0)))) == IRInteger(1)


def test_pow_one_exponent(vm: VM) -> None:
    assert vm.eval(IRApply(POW, (X, IRInteger(1)))) == X


def test_neg_neg_cancels(vm: VM) -> None:
    expr = IRApply(NEG, (IRApply(NEG, (X,)),))
    assert vm.eval(expr) == X


# ---------------------------------------------------------------------------
# Numeric folding still works with symbols around
# ---------------------------------------------------------------------------


def test_numeric_folds_inside_symbolic(vm: VM) -> None:
    # Add(x, Add(2, 3)) → Add(x, 5). Inner numeric part folds; outer
    # stays because x is free.
    inner = IRApply(ADD, (IRInteger(2), IRInteger(3)))
    outer = IRApply(ADD, (X, inner))
    assert vm.eval(outer) == IRApply(ADD, (X, IRInteger(5)))


def test_equal_of_same_symbol(vm: VM) -> None:
    assert vm.eval(IRApply(IRSymbol("Equal"), (X, X))) == IRSymbol("True")


# ---------------------------------------------------------------------------
# Elementary functions keep symbolic args
# ---------------------------------------------------------------------------


def test_sin_symbolic_stays(vm: VM) -> None:
    expr = IRApply(SIN, (X,))
    assert vm.eval(expr) == expr


def test_sin_zero_folds(vm: VM) -> None:
    assert vm.eval(IRApply(SIN, (IRInteger(0),))) == IRInteger(0)


# ---------------------------------------------------------------------------
# Differentiation
# ---------------------------------------------------------------------------


def test_diff_constant(vm: VM) -> None:
    assert vm.eval(IRApply(D, (IRInteger(7), X))) == IRInteger(0)


def test_diff_var_wrt_self(vm: VM) -> None:
    assert vm.eval(IRApply(D, (X, X))) == IRInteger(1)


def test_diff_other_var(vm: VM) -> None:
    assert vm.eval(IRApply(D, (Y, X))) == IRInteger(0)


def test_diff_sum(vm: VM) -> None:
    # d/dx (x + 3) = 1
    expr = IRApply(D, (IRApply(ADD, (X, IRInteger(3))), X))
    assert vm.eval(expr) == IRInteger(1)


def test_diff_sub(vm: VM) -> None:
    # d/dx (x - 3) = 1 (via sum rule; 1 - 0 = 1)
    expr = IRApply(D, (IRApply(SUB, (X, IRInteger(3))), X))
    assert vm.eval(expr) == IRInteger(1)


def test_diff_product(vm: VM) -> None:
    # d/dx (x * 5) = 5
    expr = IRApply(D, (IRApply(MUL, (X, IRInteger(5))), X))
    assert vm.eval(expr) == IRInteger(5)


def test_diff_power(vm: VM) -> None:
    # d/dx (x^2) = 2*x
    expr = IRApply(D, (IRApply(POW, (X, IRInteger(2))), X))
    result = vm.eval(expr)
    # Could come out as Mul(2, x) or Mul(Mul(2, x), 1) depending on
    # how identities collapse. What we require: it simplifies to
    # IRApply(MUL, (IRInteger(2), X)).
    assert result == IRApply(MUL, (IRInteger(2), X))


def test_diff_sin(vm: VM) -> None:
    # d/dx sin(x) = cos(x)
    expr = IRApply(D, (IRApply(SIN, (X,)), X))
    assert vm.eval(expr) == IRApply(COS, (X,))


def test_diff_cos(vm: VM) -> None:
    # d/dx cos(x) = -sin(x)
    expr = IRApply(D, (IRApply(COS, (X,)), X))
    assert vm.eval(expr) == IRApply(NEG, (IRApply(SIN, (X,)),))


def test_diff_exp(vm: VM) -> None:
    # d/dx exp(x) = exp(x)
    expr = IRApply(D, (IRApply(EXP, (X,)), X))
    assert vm.eval(expr) == IRApply(EXP, (X,))


def test_diff_log_chain(vm: VM) -> None:
    # d/dx log(2*x) = 2/(2*x)  (we don't simplify further than that)
    expr = IRApply(D, (IRApply(LOG, (IRApply(MUL, (IRInteger(2), X)),)), X))
    result = vm.eval(expr)
    expected = IRApply(DIV, (IRInteger(2), IRApply(MUL, (IRInteger(2), X))))
    assert result == expected


def test_diff_quotient(vm: VM) -> None:
    # d/dx (x / 2) = 1/2
    expr = IRApply(D, (IRApply(DIV, (X, IRInteger(2))), X))
    from symbolic_ir import IRRational

    assert vm.eval(expr) == IRRational(1, 2)


def test_diff_nested_power(vm: VM) -> None:
    # d/dx (x^3) = 3*x^2
    expr = IRApply(D, (IRApply(POW, (X, IRInteger(3))), X))
    result = vm.eval(expr)
    expected = IRApply(MUL, (IRInteger(3), IRApply(POW, (X, IRInteger(2)))))
    assert result == expected


# ---------------------------------------------------------------------------
# Assignment and definition
# ---------------------------------------------------------------------------


def test_symbolic_assign(vm: VM) -> None:
    from symbolic_ir import ASSIGN

    vm.eval(IRApply(ASSIGN, (IRSymbol("a"), IRInteger(5))))
    assert vm.eval(IRSymbol("a")) == IRInteger(5)


def test_symbolic_function_composition(vm: VM) -> None:
    # f(t) := t^2; f(3) + f(4) → 9 + 16 = 25
    from symbolic_ir import DEFINE, LIST

    vm.eval(
        IRApply(
            DEFINE,
            (
                IRSymbol("f"),
                IRApply(LIST, (IRSymbol("t"),)),
                IRApply(POW, (IRSymbol("t"), IRInteger(2))),
            ),
        )
    )
    total = IRApply(
        ADD,
        (
            IRApply(IRSymbol("f"), (IRInteger(3),)),
            IRApply(IRSymbol("f"), (IRInteger(4),)),
        ),
    )
    assert vm.eval(total) == IRInteger(25)


# ---------------------------------------------------------------------------
# Logic short-circuits
# ---------------------------------------------------------------------------


def test_and_short_circuits_on_false(vm: VM) -> None:
    from symbolic_ir import AND

    TRUE = IRSymbol("True")
    FALSE = IRSymbol("False")
    assert vm.eval(IRApply(AND, (TRUE, FALSE, X))) == FALSE


def test_or_short_circuits_on_true(vm: VM) -> None:
    from symbolic_ir import OR

    TRUE = IRSymbol("True")
    assert vm.eval(IRApply(OR, (X, TRUE))) == TRUE
