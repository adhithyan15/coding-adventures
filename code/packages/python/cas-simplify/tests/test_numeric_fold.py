"""Numeric folding inside Add and Mul."""

from __future__ import annotations

from symbolic_ir import ADD, MUL, IRApply, IRFloat, IRInteger, IRRational, IRSymbol

from cas_simplify import numeric_fold


def test_fold_integers_in_add() -> None:
    """Add(2, 3, x) → Add(5, x)."""
    expr = IRApply(ADD, (IRInteger(2), IRInteger(3), IRSymbol("x")))
    out = numeric_fold(expr)
    assert out == IRApply(ADD, (IRInteger(5), IRSymbol("x")))


def test_fold_integers_in_mul() -> None:
    """Mul(2, 3, x) → Mul(6, x)."""
    expr = IRApply(MUL, (IRInteger(2), IRInteger(3), IRSymbol("x")))
    out = numeric_fold(expr)
    assert out == IRApply(MUL, (IRInteger(6), IRSymbol("x")))


def test_fold_to_zero_drops_in_add_with_others() -> None:
    """Add(0, x) → x."""
    expr = IRApply(ADD, (IRInteger(0), IRSymbol("x")))
    out = numeric_fold(expr)
    assert out == IRSymbol("x")


def test_fold_to_one_drops_in_mul_with_others() -> None:
    """Mul(1, x) → x."""
    expr = IRApply(MUL, (IRInteger(1), IRSymbol("x")))
    out = numeric_fold(expr)
    assert out == IRSymbol("x")


def test_fold_rationals() -> None:
    """Add(1/2, 1/3, x) → Add(5/6, x)."""
    expr = IRApply(
        ADD, (IRRational(1, 2), IRRational(1, 3), IRSymbol("x"))
    )
    out = numeric_fold(expr)
    assert out == IRApply(ADD, (IRRational(5, 6), IRSymbol("x")))


def test_fold_with_floats_promotes() -> None:
    """A single float contaminates the result."""
    expr = IRApply(ADD, (IRFloat(1.5), IRInteger(2), IRSymbol("x")))
    out = numeric_fold(expr)
    assert isinstance(out, IRApply)
    head_args = out.args
    assert isinstance(head_args[0], IRFloat)
    assert abs(head_args[0].value - 3.5) < 1e-9


def test_only_constants_collapses() -> None:
    """Add(2, 3) → 5 (singleton drops via the head check)."""
    expr = IRApply(ADD, (IRInteger(2), IRInteger(3)))
    assert numeric_fold(expr) == IRInteger(5)


def test_fold_rational_to_integer() -> None:
    """Add(1/2, 1/2) → 1 — rational result reduces to integer."""
    expr = IRApply(ADD, (IRRational(1, 2), IRRational(1, 2)))
    assert numeric_fold(expr) == IRInteger(1)


def test_no_fold_when_no_literals() -> None:
    """Add(a, b) — no numerics, unchanged."""
    expr = IRApply(ADD, (IRSymbol("a"), IRSymbol("b")))
    assert numeric_fold(expr) == expr
