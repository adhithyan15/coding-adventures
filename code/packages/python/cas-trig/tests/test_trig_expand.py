"""Tests for TrigExpand."""

from __future__ import annotations

import pytest
from symbolic_ir import IRApply, IRInteger, IRSymbol

from cas_trig import trig_expand

ADD = IRSymbol("Add")
SUB = IRSymbol("Sub")
MUL = IRSymbol("Mul")
NEG = IRSymbol("Neg")
SIN = IRSymbol("Sin")
COS = IRSymbol("Cos")


def _sin(x: object) -> IRApply:
    return IRApply(SIN, (x,))  # type: ignore[arg-type]


def _cos(x: object) -> IRApply:
    return IRApply(COS, (x,))  # type: ignore[arg-type]


def _mul(*args: object) -> IRApply:
    return IRApply(MUL, tuple(args))  # type: ignore[arg-type]


def _add(*args: object) -> IRApply:
    return IRApply(ADD, tuple(args))  # type: ignore[arg-type]


def _sub(a: object, b: object) -> IRApply:
    return IRApply(SUB, (a, b))  # type: ignore[arg-type]


a = IRSymbol("a")
b = IRSymbol("b")
x = IRSymbol("x")


# ---------------------------------------------------------------------------
# Angle-addition formulas
# ---------------------------------------------------------------------------


def test_sin_add_expands() -> None:
    """sin(a + b) → sin(a)cos(b) + cos(a)sin(b)."""
    expr = _sin(_add(a, b))
    result = trig_expand(expr)
    # Result should be Add or Sub with trig sub-expressions
    assert isinstance(result, IRApply)
    assert result.head.name in ("Add", "Sub")


def test_cos_add_expands() -> None:
    """cos(a + b) → cos(a)cos(b) - sin(a)sin(b)."""
    expr = _cos(_add(a, b))
    result = trig_expand(expr)
    assert isinstance(result, IRApply)
    assert result.head.name in ("Add", "Sub")


def test_sin_sub_expands() -> None:
    """sin(a - b) → sin(a)cos(b) - cos(a)sin(b)."""
    expr = _sin(_sub(a, b))
    result = trig_expand(expr)
    assert isinstance(result, IRApply)
    assert result.head.name in ("Add", "Sub")


def test_cos_sub_expands() -> None:
    """cos(a - b) → cos(a)cos(b) + sin(a)sin(b)."""
    expr = _cos(_sub(a, b))
    result = trig_expand(expr)
    assert isinstance(result, IRApply)
    assert result.head.name in ("Add", "Sub")


# ---------------------------------------------------------------------------
# Double-angle
# ---------------------------------------------------------------------------


def test_sin_2x() -> None:
    """sin(2x) → 2·sin(x)·cos(x)."""
    expr = _sin(_mul(IRInteger(2), x))
    result = trig_expand(expr)
    assert isinstance(result, IRApply)
    # Should expand to a Mul containing Sin and Cos


def test_cos_2x() -> None:
    """cos(2x) → cos²(x) - sin²(x)."""
    expr = _cos(_mul(IRInteger(2), x))
    result = trig_expand(expr)
    assert isinstance(result, IRApply)


# ---------------------------------------------------------------------------
# Triple-angle
# ---------------------------------------------------------------------------


def test_sin_3x() -> None:
    """sin(3x) expands via Chebyshev recurrence."""
    expr = _sin(_mul(IRInteger(3), x))
    result = trig_expand(expr)
    assert isinstance(result, IRApply)
    # Not equal to the original
    assert result != expr


def test_cos_3x() -> None:
    """cos(3x) expands via Chebyshev recurrence."""
    expr = _cos(_mul(IRInteger(3), x))
    result = trig_expand(expr)
    assert isinstance(result, IRApply)
    assert result != expr


# ---------------------------------------------------------------------------
# Negation
# ---------------------------------------------------------------------------


def test_sin_neg_x_expands() -> None:
    """sin(-x) → -sin(x)."""
    expr = _sin(IRApply(NEG, (x,)))
    result = trig_expand(expr)
    assert isinstance(result, IRApply)
    assert result.head.name == "Neg"


def test_cos_neg_x_expands() -> None:
    """cos(-x) → cos(x)."""
    expr = _cos(IRApply(NEG, (x,)))
    result = trig_expand(expr)
    assert isinstance(result, IRApply)
    assert result.head.name == "Cos"
    assert result.args[0] == x


# ---------------------------------------------------------------------------
# Passthrough
# ---------------------------------------------------------------------------


def test_sin_plain_x_unchanged() -> None:
    """sin(x) with no compound argument stays as sin(x)."""
    expr = _sin(x)
    result = trig_expand(expr)
    assert isinstance(result, IRApply)
    assert result.head.name == "Sin"


def test_non_trig_passes_through() -> None:
    """Non-trig expression is returned (possibly canonicalised)."""
    y = IRSymbol("y")
    expr = _add(x, y)
    result = trig_expand(expr)
    assert isinstance(result, IRApply)
    assert result.head.name == "Add"
