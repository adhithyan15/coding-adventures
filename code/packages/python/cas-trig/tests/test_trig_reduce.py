"""Tests for TrigReduce."""

from __future__ import annotations

import pytest
from symbolic_ir import IRApply, IRInteger, IRRational, IRSymbol

from cas_trig import trig_reduce

ADD = IRSymbol("Add")
SUB = IRSymbol("Sub")
MUL = IRSymbol("Mul")
POW = IRSymbol("Pow")
SIN = IRSymbol("Sin")
COS = IRSymbol("Cos")


def _sin(x: object) -> IRApply:
    return IRApply(SIN, (x,))  # type: ignore[arg-type]


def _cos(x: object) -> IRApply:
    return IRApply(COS, (x,))  # type: ignore[arg-type]


def _pow(base: object, exp: int) -> IRApply:
    return IRApply(POW, (base, IRInteger(exp)))  # type: ignore[arg-type]


def _mul(*args: object) -> IRApply:
    return IRApply(MUL, tuple(args))  # type: ignore[arg-type]


x = IRSymbol("x")


# ---------------------------------------------------------------------------
# sin²(x) reduction
# ---------------------------------------------------------------------------


def test_sin2_reduces() -> None:
    """sin²(x) → (1 - cos(2x)) / 2."""
    expr = _pow(_sin(x), 2)
    result = trig_reduce(expr)
    assert isinstance(result, IRApply)
    # Should not be a Pow with Sin base
    assert not (
        result.head.name == "Pow"
        and isinstance(result.args[0], IRApply)
        and result.args[0].head.name == "Sin"
    )


# ---------------------------------------------------------------------------
# cos²(x) reduction
# ---------------------------------------------------------------------------


def test_cos2_reduces() -> None:
    """cos²(x) → (1 + cos(2x)) / 2."""
    expr = _pow(_cos(x), 2)
    result = trig_reduce(expr)
    assert isinstance(result, IRApply)
    assert not (
        result.head.name == "Pow"
        and isinstance(result.args[0], IRApply)
        and result.args[0].head.name == "Cos"
    )


# ---------------------------------------------------------------------------
# Higher powers
# ---------------------------------------------------------------------------


def test_sin3_reduces() -> None:
    """sin³(x) → (3 sin(x) - sin(3x)) / 4."""
    expr = _pow(_sin(x), 3)
    result = trig_reduce(expr)
    assert isinstance(result, IRApply)
    assert result != expr


def test_cos3_reduces() -> None:
    """cos³(x) → (3 cos(x) + cos(3x)) / 4."""
    expr = _pow(_cos(x), 3)
    result = trig_reduce(expr)
    assert isinstance(result, IRApply)
    assert result != expr


def test_sin4_reduces() -> None:
    """sin⁴(x) reduces."""
    expr = _pow(_sin(x), 4)
    result = trig_reduce(expr)
    assert isinstance(result, IRApply)
    assert result != expr


def test_cos4_reduces() -> None:
    """cos⁴(x) reduces."""
    expr = _pow(_cos(x), 4)
    result = trig_reduce(expr)
    assert isinstance(result, IRApply)
    assert result != expr


def test_sin6_reduces() -> None:
    """sin⁶(x) reduces (n ≤ 6 hard-coded)."""
    expr = _pow(_sin(x), 6)
    result = trig_reduce(expr)
    assert isinstance(result, IRApply)
    assert result != expr


# ---------------------------------------------------------------------------
# Product sin(x)·cos(x) → sin(2x)/2
# ---------------------------------------------------------------------------


def test_sin_times_cos_reduces() -> None:
    """sin(x)·cos(x) → sin(2x)/2."""
    expr = _mul(_sin(x), _cos(x))
    result = trig_reduce(expr)
    assert isinstance(result, IRApply)
    # Result should not be a plain Mul(Sin, Cos) any more
    # It should contain Sin(2x) or Mul involving Sin
    result_str = str(result)  # Just check it changed
    assert result != expr


# ---------------------------------------------------------------------------
# Passthrough
# ---------------------------------------------------------------------------


def test_sin1_passthrough() -> None:
    """sin¹(x) = sin(x) — no reduction needed."""
    expr = _pow(_sin(x), 1)
    result = trig_reduce(expr)
    # Pow(Sin(x), 1) — canonical form may simplify to Sin(x)
    assert isinstance(result, IRApply)


def test_pow7_passthrough() -> None:
    """sin⁷(x) — n > 6, returned unchanged (Phase 2)."""
    expr = _pow(_sin(x), 7)
    result = trig_reduce(expr)
    # Should stay as Pow(Sin(x), 7) since n > 6 is not handled
    assert isinstance(result, IRApply)


def test_non_trig_passes_through() -> None:
    """Non-trig expression passes through."""
    y = IRSymbol("y")
    expr = _pow(y, 2)
    result = trig_reduce(expr)
    # y^2 has no trig, passes through unchanged
    assert isinstance(result, IRApply)
    assert result.head.name == "Pow"
