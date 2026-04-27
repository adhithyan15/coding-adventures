"""Rational-root test."""

from __future__ import annotations

from cas_factor import extract_linear_factors, find_integer_roots


def test_find_roots_simple_quadratic() -> None:
    """x^2 - 1 → roots = {-1, 1}."""
    assert sorted(find_integer_roots([-1, 0, 1])) == [-1, 1]


def test_find_roots_cubic() -> None:
    """x^3 - 6x^2 + 11x - 6 = (x-1)(x-2)(x-3)."""
    p = [-6, 11, -6, 1]
    assert sorted(find_integer_roots(p)) == [1, 2, 3]


def test_find_roots_irreducible_quadratic() -> None:
    """x^2 + 1 has no integer (or rational) roots."""
    assert find_integer_roots([1, 0, 1]) == []


def test_find_roots_zero_constant() -> None:
    """x*(x-1) → roots = {0, 1}."""
    p = [0, -1, 1]
    assert sorted(find_integer_roots(p)) == [0, 1]


# ---- extract_linear_factors ------------------------------------------------


def test_extract_simple() -> None:
    """x^2 - 1 → factors=[(-1,1),(1,1)], residual=[1]."""
    factors, residual = extract_linear_factors([-1, 0, 1])
    assert factors == [(-1, 1), (1, 1)]
    assert residual == [1]


def test_extract_with_multiplicity() -> None:
    """(x+1)^2 = x^2 + 2x + 1 → factors=[(-1, 2)], residual=[1]."""
    factors, residual = extract_linear_factors([1, 2, 1])
    assert factors == [(-1, 2)]
    assert residual == [1]


def test_extract_leaves_irreducible() -> None:
    """x^2 + 1 has no linear factors → residual is the input."""
    factors, residual = extract_linear_factors([1, 0, 1])
    assert factors == []
    assert residual == [1, 0, 1]


def test_extract_mixed() -> None:
    """(x-1)(x^2+1) = x^3 - x^2 + x - 1.

    Should pull out (x-1), leaving (x^2 + 1) as residual.
    """
    p = [-1, 1, -1, 1]
    factors, residual = extract_linear_factors(p)
    assert factors == [(1, 1)]
    assert residual == [1, 0, 1]
