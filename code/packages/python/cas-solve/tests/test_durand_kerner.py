"""Tests for Durand–Kerner numeric root-finding."""

from __future__ import annotations

import cmath
import math
from fractions import Fraction

import pytest
from symbolic_ir import IRApply, IRFloat, IRSymbol

from cas_solve import nsolve_fraction_poly, nsolve_poly, roots_to_ir


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _assert_roots_close(
    computed: list[complex], expected: list[complex], tol: float = 1e-8
) -> None:
    """Assert that ``computed`` matches ``expected`` (up to permutation)."""
    assert len(computed) == len(expected)
    used = [False] * len(expected)
    for z in computed:
        matched = False
        for j, w in enumerate(expected):
            if not used[j] and abs(z - w) < tol:
                used[j] = True
                matched = True
                break
        assert matched, f"Root {z} not matched in {expected}"


# ---------------------------------------------------------------------------
# nsolve_poly — basic tests
# ---------------------------------------------------------------------------


def test_linear_x_minus_2() -> None:
    """x - 2 = 0 → root 2."""
    roots = nsolve_poly([1.0, -2.0])
    assert len(roots) == 1
    assert abs(roots[0] - 2.0) < 1e-10


def test_quadratic_x2_minus_1() -> None:
    """x² - 1 = 0 → roots ±1."""
    roots = nsolve_poly([1.0, 0.0, -1.0])
    _assert_roots_close(roots, [1.0, -1.0])


def test_quadratic_x2_plus_1() -> None:
    """x² + 1 = 0 → roots ±i."""
    roots = nsolve_poly([1.0, 0.0, 1.0])
    _assert_roots_close(roots, [1j, -1j])


def test_cubic_three_real() -> None:
    """(x-1)(x-2)(x-3) = x³ - 6x² + 11x - 6 → roots 1, 2, 3."""
    roots = nsolve_poly([1.0, -6.0, 11.0, -6.0])
    _assert_roots_close(roots, [1.0, 2.0, 3.0])


def test_quintic_five_roots() -> None:
    """x⁵ - 1 = 0 → 5 roots (the 5th roots of unity)."""
    roots = nsolve_poly([1.0, 0.0, 0.0, 0.0, 0.0, -1.0])
    assert len(roots) == 5
    # All roots have |z| ≈ 1
    for z in roots:
        assert abs(abs(z) - 1.0) < 1e-8


def test_degree_zero_returns_empty() -> None:
    """Degree 0 polynomial (constant): no roots."""
    roots = nsolve_poly([5.0])
    assert roots == []


def test_roots_match_vieta() -> None:
    """Sum of roots ≈ -a_{n-1} (Vieta's formula), product ≈ a_0 (for monic)."""
    # x³ - 6x² + 11x - 6 → sum = 6, product = 6
    roots = nsolve_poly([1.0, -6.0, 11.0, -6.0])
    assert abs(sum(roots).real - 6.0) < 1e-8
    prod = 1.0
    for r in roots:
        prod *= r
    assert abs(prod.real - 6.0) < 1e-8


# ---------------------------------------------------------------------------
# roots_to_ir — IR node conversion
# ---------------------------------------------------------------------------


def test_roots_to_ir_real_roots() -> None:
    """Near-real roots become IRFloat nodes."""
    ir_roots = roots_to_ir([complex(2.0, 1e-15), complex(-3.0, -1e-14)])
    assert all(isinstance(r, IRFloat) for r in ir_roots)
    vals = {r.value for r in ir_roots}  # type: ignore[union-attr]
    assert abs(list(vals)[0] - 2.0) < 1e-12 or abs(list(vals)[0] + 3.0) < 1e-12


def test_roots_to_ir_complex_root() -> None:
    """Truly complex roots become Add(IRFloat, Mul(IRFloat, %i)) nodes."""
    ir_roots = roots_to_ir([complex(1.0, 2.0)])
    assert len(ir_roots) == 1
    r = ir_roots[0]
    assert isinstance(r, IRApply)
    assert r.head.name == "Add"


def test_roots_to_ir_pure_imaginary() -> None:
    """0 + i → Add(IRFloat(0), Mul(IRFloat(1), %i))."""
    ir_roots = roots_to_ir([complex(0.0, 1.0)])
    assert len(ir_roots) == 1
    # Could be Add or IRFloat depending on real part threshold
    # Just verify it's an IRApply (complex) since im = 1 >> threshold
    r = ir_roots[0]
    assert isinstance(r, IRApply)


# ---------------------------------------------------------------------------
# nsolve_fraction_poly — Fraction coefficient wrapper
# ---------------------------------------------------------------------------


def test_nsolve_fraction_poly_cubic() -> None:
    """Fraction coefficients: x³ - 6x² + 11x - 6 → roots ≈ 1, 2, 3."""
    from fractions import Fraction

    coeffs = [Fraction(1), Fraction(-6), Fraction(11), Fraction(-6)]
    ir_roots = nsolve_fraction_poly(coeffs)
    assert len(ir_roots) == 3
    vals = sorted(r.value for r in ir_roots if isinstance(r, IRFloat))
    assert len(vals) == 3
    assert abs(vals[0] - 1.0) < 1e-8
    assert abs(vals[1] - 2.0) < 1e-8
    assert abs(vals[2] - 3.0) < 1e-8


def test_nsolve_fraction_poly_quintic() -> None:
    """x⁵ + x + 1 has 5 roots, all close to the unit disk."""
    from fractions import Fraction

    # x^5 + x + 1 = 0
    coeffs = [Fraction(1), Fraction(0), Fraction(0), Fraction(0), Fraction(1), Fraction(1)]
    ir_roots = nsolve_fraction_poly(coeffs)
    assert len(ir_roots) == 5
