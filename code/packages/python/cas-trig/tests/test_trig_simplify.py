"""Tests for TrigSimplify."""

from __future__ import annotations

import pytest
from symbolic_ir import IRApply, IRInteger, IRRational, IRSymbol

from cas_trig import trig_simplify

ADD = IRSymbol("Add")
SUB = IRSymbol("Sub")
MUL = IRSymbol("Mul")
POW = IRSymbol("Pow")
NEG = IRSymbol("Neg")
SIN = IRSymbol("Sin")
COS = IRSymbol("Cos")
TAN = IRSymbol("Tan")
PI = IRSymbol("%pi")


def _pow2(node: IRApply) -> IRApply:
    return IRApply(POW, (node, IRInteger(2)))


def _sin(x: object) -> IRApply:
    return IRApply(SIN, (x,))  # type: ignore[arg-type]


def _cos(x: object) -> IRApply:
    return IRApply(COS, (x,))  # type: ignore[arg-type]


x = IRSymbol("x")


# ---------------------------------------------------------------------------
# Pythagorean identity
# ---------------------------------------------------------------------------


def test_sin2_plus_cos2_equals_1() -> None:
    """sin²(x) + cos²(x) → 1."""
    expr = IRApply(ADD, (_pow2(_sin(x)), _pow2(_cos(x))))
    result = trig_simplify(expr)
    assert result == IRInteger(1)


def test_cos2_plus_sin2_equals_1() -> None:
    """cos²(x) + sin²(x) → 1 (reordered)."""
    expr = IRApply(ADD, (_pow2(_cos(x)), _pow2(_sin(x))))
    result = trig_simplify(expr)
    assert result == IRInteger(1)


# ---------------------------------------------------------------------------
# Sign / parity rules
# ---------------------------------------------------------------------------


def test_sin_neg_x() -> None:
    """sin(-x) → -sin(x)."""
    neg_x = IRApply(NEG, (x,))
    expr = _sin(neg_x)
    result = trig_simplify(expr)
    assert isinstance(result, IRApply)
    assert result.head.name == "Neg"
    inner = result.args[0]
    assert isinstance(inner, IRApply)
    assert inner.head.name == "Sin"


def test_cos_neg_x() -> None:
    """cos(-x) → cos(x)."""
    neg_x = IRApply(NEG, (x,))
    expr = _cos(neg_x)
    result = trig_simplify(expr)
    assert isinstance(result, IRApply)
    assert result.head.name == "Cos"
    assert result.args[0] == x


# ---------------------------------------------------------------------------
# Special values
# ---------------------------------------------------------------------------


def test_sin_pi_is_zero() -> None:
    """sin(π) → 0."""
    result = trig_simplify(_sin(PI))
    assert result == IRInteger(0)


def test_cos_pi_is_neg_one() -> None:
    """cos(π) → -1."""
    result = trig_simplify(_cos(PI))
    assert result == IRInteger(-1)


def test_sin_pi_over_6() -> None:
    """sin(π/6) → 1/2."""
    arg = IRApply(MUL, (IRRational(1, 6), PI))
    result = trig_simplify(_sin(arg))
    assert result == IRRational(1, 2)


def test_cos_pi_over_2_is_zero() -> None:
    """cos(π/2) → 0."""
    arg = IRApply(MUL, (IRRational(1, 2), PI))
    result = trig_simplify(_cos(arg))
    assert result == IRInteger(0)


def test_sin_pi_over_2_is_one() -> None:
    """sin(π/2) → 1."""
    arg = IRApply(MUL, (IRRational(1, 2), PI))
    result = trig_simplify(_sin(arg))
    assert result == IRInteger(1)


# ---------------------------------------------------------------------------
# Idempotence and non-trig passthrough
# ---------------------------------------------------------------------------


def test_trig_simplify_idempotent() -> None:
    """Applying trig_simplify twice gives the same result."""
    expr = IRApply(ADD, (_pow2(_sin(x)), _pow2(_cos(x))))
    first = trig_simplify(expr)
    second = trig_simplify(first)
    assert first == second


def test_non_trig_passes_through() -> None:
    """An expression with no trig functions is returned unchanged."""
    y = IRSymbol("y")
    expr = IRApply(ADD, (x, y))
    result = trig_simplify(expr)
    # Should be canonically equivalent (canonical may reorder but not change value)
    assert isinstance(result, IRApply)
    assert result.head.name == "Add"
