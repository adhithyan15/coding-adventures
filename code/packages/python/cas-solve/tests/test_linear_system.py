"""Tests for linear-system solver (Gaussian elimination)."""

from __future__ import annotations

from fractions import Fraction

import pytest
from symbolic_ir import IRApply, IRInteger, IRRational, IRSymbol

from cas_solve import solve_linear_system


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

X = IRSymbol("x")
Y = IRSymbol("y")
Z = IRSymbol("z")
EQ = IRSymbol("Equal")
ADD = IRSymbol("Add")
SUB = IRSymbol("Sub")
MUL = IRSymbol("Mul")


def _int(n: int) -> IRInteger:
    return IRInteger(n)


def _eq(lhs: object, rhs: object) -> IRApply:
    return IRApply(EQ, (lhs, rhs))  # type: ignore[arg-type]


def _add(*args: object) -> IRApply:
    return IRApply(ADD, tuple(args))  # type: ignore[arg-type]


def _sub(a: object, b: object) -> IRApply:
    return IRApply(SUB, (a, b))  # type: ignore[arg-type]


def _mul(a: object, b: object) -> IRApply:
    return IRApply(MUL, (a, b))  # type: ignore[arg-type]


def _rule_value(rules: list, var: IRSymbol) -> object:
    """Return the value in Rule(var, value) from the list."""
    for r in rules:
        assert isinstance(r, IRApply) and r.head.name == "Rule"
        if r.args[0] == var:
            return r.args[1]
    raise KeyError(f"No rule for {var}")


# ---------------------------------------------------------------------------
# Basic 2×2 systems
# ---------------------------------------------------------------------------


def test_system_2x2_simple() -> None:
    """x + y = 3, x - y = 1  →  x=2, y=1."""
    # x + y - 3 = 0, x - y - 1 = 0
    eq1 = _eq(_add(X, Y), _int(3))
    eq2 = _eq(_sub(X, Y), _int(1))
    result = solve_linear_system([eq1, eq2], [X, Y])
    assert result is not None
    assert _rule_value(result, X) == _int(2)
    assert _rule_value(result, Y) == _int(1)


def test_system_2x2_rational_solution() -> None:
    """2x + 3y = 7, 4x - y = 1  →  x = 5/7, y = 13/7."""
    # Equations in Equal() form
    eq1 = _eq(_add(_mul(_int(2), X), _mul(_int(3), Y)), _int(7))
    eq2 = _eq(_sub(_mul(_int(4), X), Y), _int(1))
    result = solve_linear_system([eq1, eq2], [X, Y])
    assert result is not None
    xval = _rule_value(result, X)
    yval = _rule_value(result, Y)
    assert isinstance(xval, IRRational)
    assert xval.numer == 5 and xval.denom == 7
    assert isinstance(yval, IRRational)
    assert yval.numer == 13 and yval.denom == 7


def test_system_2x2_single_equation() -> None:
    """Wrong count of equations → None."""
    eq1 = _eq(_add(X, Y), _int(3))
    result = solve_linear_system([eq1], [X, Y])
    assert result is None


def test_system_empty() -> None:
    """Empty system → None."""
    result = solve_linear_system([], [])
    assert result is None


# ---------------------------------------------------------------------------
# 3×3 system
# ---------------------------------------------------------------------------


def test_system_3x3() -> None:
    """x + y + z = 6, 2x + y = 5, z = 3  →  x=2, y=1, z=3."""
    # Verify by hand: z=3, y=5-2x, x+(5-2x)+3=6 → -x=-2 → x=2, y=1.
    eq1 = _eq(_add(X, _add(Y, Z)), _int(6))   # x + y + z = 6
    eq2 = _eq(_add(_mul(_int(2), X), Y), _int(5))  # 2x + y = 5
    eq3 = _eq(Z, _int(3))  # z = 3
    result = solve_linear_system([eq1, eq2, eq3], [X, Y, Z])
    assert result is not None
    assert _rule_value(result, X) == _int(2)
    assert _rule_value(result, Y) == _int(1)
    assert _rule_value(result, Z) == _int(3)


# ---------------------------------------------------------------------------
# Degenerate / non-linear cases
# ---------------------------------------------------------------------------


def test_system_singular() -> None:
    """Singular system (parallel lines) → None."""
    # x + y = 1, 2x + 2y = 2 → infinite solutions
    eq1 = _eq(_add(X, Y), _int(1))
    eq2 = _eq(_add(_mul(_int(2), X), _mul(_int(2), Y)), _int(2))
    result = solve_linear_system([eq1, eq2], [X, Y])
    assert result is None


def test_system_non_linear_pow() -> None:
    """x^2 = 4 is non-linear → None."""
    from symbolic_ir import IRApply, IRInteger, IRSymbol

    POW = IRSymbol("Pow")
    eq = IRApply(EQ, (IRApply(POW, (X, IRInteger(2))), _int(4)))
    result = solve_linear_system([eq], [X])
    assert result is None


def test_system_returns_rule_nodes() -> None:
    """Result is a list of Rule(var, val) IRApply nodes."""
    eq1 = _eq(_add(X, Y), _int(3))
    eq2 = _eq(_sub(X, Y), _int(1))
    result = solve_linear_system([eq1, eq2], [X, Y])
    assert result is not None
    assert len(result) == 2
    for r in result:
        assert isinstance(r, IRApply)
        assert r.head.name == "Rule"


def test_system_1x1() -> None:
    """1×1 system: 3x = 9 → x = 3."""
    eq = _eq(_mul(_int(3), X), _int(9))
    result = solve_linear_system([eq], [X])
    assert result is not None
    assert _rule_value(result, X) == _int(3)


# ---------------------------------------------------------------------------
# Zero-form equations (expr = 0)
# ---------------------------------------------------------------------------


def test_system_zero_form() -> None:
    """Equations as plain IR (not Equal), treated as = 0."""
    # x + y (= 0): x = -y ... combine with x - y (= 0) → x = 0, y = 0
    eq1 = _add(X, Y)   # x + y = 0
    eq2 = _sub(X, Y)   # x - y = 0
    result = solve_linear_system([eq1, eq2], [X, Y])
    assert result is not None
    assert _rule_value(result, X) == _int(0)
    assert _rule_value(result, Y) == _int(0)
