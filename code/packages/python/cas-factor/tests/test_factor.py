"""factor_integer_polynomial — orchestrator tests (Phase 1 + Kronecker)."""

from __future__ import annotations

from cas_factor import factor_integer_polynomial
from cas_factor.polynomial import evaluate

# ---------------------------------------------------------------------------
# Phase 1 regression tests
# ---------------------------------------------------------------------------


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
    """x^2 + 1 — irreducible over Z; returned as a single residual factor."""
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


# ---------------------------------------------------------------------------
# Phase 2 (Kronecker) tests
# ---------------------------------------------------------------------------


def test_factor_sophie_germain_x4_plus_4() -> None:
    """x^4 + 4 = (x^2 + 2x + 2)(x^2 - 2x + 2) — Sophie Germain identity."""
    content, factors = factor_integer_polynomial([4, 0, 0, 0, 1])
    assert content == 1
    assert len(factors) == 2
    # Both factors should be degree-2 with multiplicity 1.
    assert all(mult == 1 for _, mult in factors)
    # Verify product at several evaluation points.
    p = [4, 0, 0, 0, 1]
    for x_val in range(-5, 6):
        product = 1
        for poly, mult in factors:
            product *= evaluate(poly, x_val) ** mult
        assert product == evaluate(p, x_val)


def test_factor_x4_plus_x2_plus_1() -> None:
    """x^4 + x^2 + 1 = (x^2 + x + 1)(x^2 - x + 1)."""
    content, factors = factor_integer_polynomial([1, 0, 1, 0, 1])
    assert content == 1
    assert len(factors) == 2
    assert all(mult == 1 for _, mult in factors)
    p = [1, 0, 1, 0, 1]
    for x_val in range(-5, 6):
        product = 1
        for poly, mult in factors:
            product *= evaluate(poly, x_val) ** mult
        assert product == evaluate(p, x_val)


def test_factor_repeated_irreducible_quadratic() -> None:
    """x^4 + 2x^2 + 1 = (x^2 + 1)^2."""
    content, factors = factor_integer_polynomial([1, 0, 2, 0, 1])
    assert content == 1
    assert len(factors) == 1
    poly, mult = factors[0]
    assert mult == 2
    assert poly == [1, 0, 1]  # x^2 + 1


def test_factor_product_of_quadratic_and_linear() -> None:
    """(x^2 + 1)(x - 2) = x^3 - 2x^2 + x - 2 = [-2, 1, -2, 1]."""
    # Polynomial: -2 + x - 2x^2 + x^3
    content, factors = factor_integer_polynomial([-2, 1, -2, 1])
    assert content == 1
    # Should find (x - 2) as linear factor, then (x^2 + 1) as residual.
    factor_map: dict[tuple[int, ...], int] = {}
    for poly, mult in factors:
        factor_map[tuple(poly)] = mult
    assert factor_map.get((-2, 1), 0) == 1  # x - 2
    assert factor_map.get((1, 0, 1), 0) == 1  # x^2 + 1


def test_factor_x6_minus_1() -> None:
    """x^6 - 1 = (x-1)(x+1)(x^2+x+1)(x^2-x+1)."""
    content, factors = factor_integer_polynomial([-1, 0, 0, 0, 0, 0, 1])
    assert content == 1
    p = [-1, 0, 0, 0, 0, 0, 1]
    for x_val in range(-5, 6):
        product = 1
        for poly, mult in factors:
            product *= evaluate(poly, x_val) ** mult
        assert product == evaluate(p, x_val)
    # Total degree should sum to 6.
    total_degree = sum(len(poly) - 1 for poly, mult in factors for _ in range(mult))
    assert total_degree == 6


def test_factor_irreducible_x_squared_plus_1_no_change() -> None:
    """Irreducible polynomial comes back unchanged."""
    content, factors = factor_integer_polynomial([1, 0, 1])
    assert content == 1
    assert len(factors) == 1
    assert factors[0] == ([1, 0, 1], 1)
