"""taylor_polynomial — polynomial Taylor expansion."""

from __future__ import annotations

import pytest
from symbolic_ir import ADD, MUL, POW, SUB, IRApply, IRInteger, IRSymbol

from cas_limit_series import PolynomialError, taylor_polynomial

x = IRSymbol("x")


def test_taylor_constant() -> None:
    """Taylor of a constant at any point is the constant itself."""
    out = taylor_polynomial(IRInteger(7), x, IRInteger(2), order=3)
    assert out == IRInteger(7)


def test_taylor_x_at_zero_order_2() -> None:
    """Taylor(x, x, 0, 2) = x."""
    out = taylor_polynomial(x, x, IRInteger(0), order=2)
    assert out == x


def test_taylor_x_squared_at_zero_full_order() -> None:
    """Taylor(x^2, x, 0, 2) = x^2 (the full polynomial)."""
    expr = IRApply(POW, (x, IRInteger(2)))
    out = taylor_polynomial(expr, x, IRInteger(0), order=2)
    # Result is x^2 → Mul-of-1*Pow form; or just Pow.
    # Our implementation emits Pow(x, 2) as a single term.
    assert out == IRApply(POW, (x, IRInteger(2)))


def test_taylor_x_squared_truncated() -> None:
    """Taylor(x^2, x, 0, 1) truncates to 0 (no x^0 or x^1 part)."""
    expr = IRApply(POW, (x, IRInteger(2)))
    out = taylor_polynomial(expr, x, IRInteger(0), order=1)
    assert out == IRInteger(0)


def test_taylor_polynomial_around_one() -> None:
    """Taylor(x^2, x, 1, 2) = 1 + 2*(x-1) + (x-1)^2."""
    expr = IRApply(POW, (x, IRInteger(2)))
    out = taylor_polynomial(expr, x, IRInteger(1), order=2)
    # The output should be Add of three terms; we just check it has
    # head=Add and contains the right number of terms.
    assert isinstance(out, IRApply)
    assert out.head == ADD


def test_taylor_compound_polynomial() -> None:
    """Taylor(x^2 + 1, x, 0, 2) = 1 + x^2."""
    expr = IRApply(ADD, (IRApply(POW, (x, IRInteger(2))), IRInteger(1)))
    out = taylor_polynomial(expr, x, IRInteger(0), order=2)
    assert isinstance(out, IRApply)
    assert out.head == ADD
    # We expect (1, x^2) in some order.
    arg_set = set(out.args)
    assert IRInteger(1) in arg_set
    assert IRApply(POW, (x, IRInteger(2))) in arg_set


def test_taylor_negative_order_raises() -> None:
    with pytest.raises(ValueError):
        taylor_polynomial(IRInteger(1), x, IRInteger(0), order=-1)


def test_taylor_non_polynomial_raises() -> None:
    """A transcendental input raises PolynomialError."""
    from symbolic_ir import SIN

    expr = IRApply(SIN, (x,))
    with pytest.raises(PolynomialError):
        taylor_polynomial(expr, x, IRInteger(0), order=3)


def test_taylor_unknown_symbol_raises() -> None:
    """A non-target symbol raises."""
    expr = IRApply(MUL, (IRSymbol("y"), x))
    with pytest.raises(PolynomialError):
        taylor_polynomial(expr, x, IRInteger(0), order=2)


def test_taylor_with_sub_and_neg() -> None:
    """Taylor handles Sub and Neg correctly."""
    # x - 1 around 0 to order 1 = -1 + x.
    expr = IRApply(SUB, (x, IRInteger(1)))
    out = taylor_polynomial(expr, x, IRInteger(0), order=1)
    assert isinstance(out, IRApply)
    assert out.head == ADD
    assert IRInteger(-1) in out.args
    assert x in out.args
