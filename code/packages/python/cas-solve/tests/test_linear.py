"""solve_linear tests."""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import IRInteger, IRRational

from cas_solve import ALL, solve_linear


def test_basic() -> None:
    """2x + 3 = 0 → x = -3/2."""
    out = solve_linear(Fraction(2), Fraction(3))
    assert out == [IRRational(-3, 2)]


def test_integer_solution() -> None:
    """x - 5 = 0 → x = 5."""
    out = solve_linear(Fraction(1), Fraction(-5))
    assert out == [IRInteger(5)]


def test_no_solution() -> None:
    """0*x + 5 = 0 → no solution."""
    assert solve_linear(Fraction(0), Fraction(5)) == []


def test_every_x_is_solution() -> None:
    """0*x + 0 = 0 → ALL."""
    assert solve_linear(Fraction(0), Fraction(0)) == ALL


def test_zero_constant() -> None:
    """3x = 0 → x = 0."""
    out = solve_linear(Fraction(3), Fraction(0))
    assert out == [IRInteger(0)]
