"""factor_integer_polynomial — orchestrator."""

from __future__ import annotations

from cas_factor import factor_integer_polynomial


def test_factor_x_squared_minus_one() -> None:
    """x^2 - 1 = (x - 1)(x + 1)."""
    content, factors = factor_integer_polynomial([-1, 0, 1])
    assert content == 1
    assert sorted(factors) == sorted([([-1, 1], 1), ([1, 1], 1)])


def test_factor_with_content() -> None:
    """2x^2 + 4x + 2 = 2 * (x + 1)^2."""
    content, factors = factor_integer_polynomial([2, 4, 2])
    assert content == 2
    assert factors == [([1, 1], 2)]


def test_factor_irreducible_quadratic() -> None:
    """x^2 + 1 — Phase 1 leaves it as a residual factor."""
    content, factors = factor_integer_polynomial([1, 0, 1])
    assert content == 1
    assert factors == [([1, 0, 1], 1)]


def test_factor_cubic() -> None:
    """x^3 - 6x^2 + 11x - 6 = (x-1)(x-2)(x-3)."""
    content, factors = factor_integer_polynomial([-6, 11, -6, 1])
    assert content == 1
    expected = [([-1, 1], 1), ([-2, 1], 1), ([-3, 1], 1)]
    assert sorted(factors) == sorted(expected)


def test_factor_with_zero_root() -> None:
    """x^3 - x = x(x-1)(x+1)."""
    content, factors = factor_integer_polynomial([0, -1, 0, 1])
    assert content == 1
    expected = [([0, 1], 1), ([-1, 1], 1), ([1, 1], 1)]
    assert sorted(factors) == sorted(expected)


def test_factor_empty() -> None:
    """Zero polynomial returns (0, [])."""
    assert factor_integer_polynomial([]) == (0, [])
