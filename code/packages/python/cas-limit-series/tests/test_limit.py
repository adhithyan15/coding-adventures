"""limit_direct — direct substitution."""

from __future__ import annotations

from symbolic_ir import ADD, MUL, POW, IRApply, IRInteger, IRSymbol

from cas_limit_series import LIMIT, limit_direct


def test_limit_polynomial_at_finite_point() -> None:
    """lim_{x→2} x^2 + 1 → Add(Pow(2,2), 1) (un-simplified)."""
    x = IRSymbol("x")
    expr = IRApply(ADD, (IRApply(POW, (x, IRInteger(2))), IRInteger(1)))
    out = limit_direct(expr, x, IRInteger(2))
    expected = IRApply(ADD, (IRApply(POW, (IRInteger(2), IRInteger(2))), IRInteger(1)))
    assert out == expected


def test_limit_substitutes_in_compound() -> None:
    """lim_{x→3} 2*x → Mul(2, 3)."""
    x = IRSymbol("x")
    expr = IRApply(MUL, (IRInteger(2), x))
    out = limit_direct(expr, x, IRInteger(3))
    assert out == IRApply(MUL, (IRInteger(2), IRInteger(3)))


def test_limit_does_not_simplify() -> None:
    """The result is intentionally un-simplified — caller does that."""
    x = IRSymbol("x")
    expr = IRApply(ADD, (x, IRInteger(0)))
    out = limit_direct(expr, x, IRInteger(5))
    # We get Add(5, 0), not 5.
    assert out == IRApply(ADD, (IRInteger(5), IRInteger(0)))


def test_limit_no_var_in_expr() -> None:
    """If var doesn't occur in expr, expr is returned unchanged."""
    x, y = IRSymbol("x"), IRSymbol("y")
    expr = IRApply(MUL, (IRInteger(2), y))
    assert limit_direct(expr, x, IRInteger(0)) == expr


def test_limit_indeterminate_returns_unevaluated() -> None:
    """A literal 0/0 returns the unevaluated Limit wrapper."""
    from symbolic_ir import DIV

    x = IRSymbol("x")
    expr = IRApply(DIV, (IRInteger(0), IRInteger(0)))
    out = limit_direct(expr, x, IRInteger(0))
    assert isinstance(out, IRApply)
    assert out.head == LIMIT
