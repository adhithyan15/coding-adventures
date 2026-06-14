"""Tests for cubic-equation solver."""

from __future__ import annotations

from fractions import Fraction

import pytest
from symbolic_ir import IRApply, IRInteger, IRRational, IRSymbol

from cas_solve import solve_cubic


def _frac(*ints: int) -> list[Fraction]:
    return [Fraction(n) for n in ints]


I_UNIT = IRSymbol("%i")


# ---------------------------------------------------------------------------
# Rational roots (rational-root theorem path)
# ---------------------------------------------------------------------------


def test_cubic_three_rational_roots() -> None:
    """x³ - 6x² + 11x - 6 = 0 → {1, 2, 3}."""
    a, b, c, d = _frac(1, -6, 11, -6)
    roots = solve_cubic(a, b, c, d)
    assert isinstance(roots, list)
    assert len(roots) == 3
    assert IRInteger(1) in roots
    assert IRInteger(2) in roots
    assert IRInteger(3) in roots


def test_cubic_one_rational_one_complex_pair() -> None:
    """x³ + 1 = 0 → {-1, (1±i√3)/2}."""
    a, b, c, d = _frac(1, 0, 0, 1)
    roots = solve_cubic(a, b, c, d)
    assert isinstance(roots, list)
    assert len(roots) == 3
    assert IRInteger(-1) in roots
    # The other two should be complex (IRApply nodes containing %i)
    complex_roots = [r for r in roots if isinstance(r, IRApply)]
    assert len(complex_roots) == 2


def test_cubic_double_root() -> None:
    """x³ - 3x - 2 = 0 → {-1, 2} (−1 is a double root, deduplicated)."""
    a, b, c, d = _frac(1, 0, -3, -2)
    roots = solve_cubic(a, b, c, d)
    assert isinstance(roots, list)
    # After deduplication: [-1, 2]
    assert IRInteger(-1) in roots
    assert IRInteger(2) in roots


def test_cubic_rational_fraction_root() -> None:
    """2x³ - 3x² - 11x + 6 = 0 → has rational roots."""
    # Roots: x = -2, 1/2, 3
    a, b, c, d = _frac(2, -3, -11, 6)
    roots = solve_cubic(a, b, c, d)
    assert isinstance(roots, list)
    # Check all 3 roots are IRInteger or IRRational, no complex
    for r in roots:
        assert isinstance(r, (IRInteger, IRRational))


def test_cubic_leading_coefficient() -> None:
    """3x³ - 6x = 0 → x(3x² - 6) = 0 → roots include 0."""
    # 3x³ + 0x² - 6x + 0 = 0 → roots 0, ±√2
    a, b, c, d = _frac(3, 0, -6, 0)
    roots = solve_cubic(a, b, c, d)
    assert isinstance(roots, list)
    assert IRInteger(0) in roots


def test_cubic_purely_imaginary_discriminant() -> None:
    """x³ - x² + x - 1 = 0 → (x-1)(x²+1) = 0 → roots: 1, ±i."""
    a, b, c, d = _frac(1, -1, 1, -1)
    roots = solve_cubic(a, b, c, d)
    assert isinstance(roots, list)
    assert IRInteger(1) in roots
    complex_roots = [r for r in roots if isinstance(r, IRApply)]
    assert len(complex_roots) == 2


def test_cubic_zero_constant() -> None:
    """x³ + x = 0 → x(x² + 1) = 0 → roots: 0, ±i."""
    a, b, c, d = _frac(1, 0, 1, 0)
    roots = solve_cubic(a, b, c, d)
    assert isinstance(roots, list)
    assert IRInteger(0) in roots


def test_cubic_delegates_to_quadratic_when_a_zero() -> None:
    """With a=0, solve_cubic delegates to solve_quadratic."""
    a, b, c, d = Fraction(0), Fraction(1), Fraction(-5), Fraction(6)
    roots = solve_cubic(a, b, c, d)
    assert isinstance(roots, list)
    assert IRInteger(2) in roots
    assert IRInteger(3) in roots


def test_cubic_returns_ir_integer_for_integer_roots() -> None:
    """All-integer-root case returns IRInteger nodes."""
    a, b, c, d = _frac(1, -3, 2, 0)  # x(x-1)(x-2) = 0
    roots = solve_cubic(a, b, c, d)
    assert isinstance(roots, list)
    int_roots = [r for r in roots if isinstance(r, IRInteger)]
    assert len(int_roots) >= 2  # at least two integer roots found


# ---------------------------------------------------------------------------
# Cardano path (no rational root)
# ---------------------------------------------------------------------------


def test_cubic_cardano_one_real_two_complex() -> None:
    """x³ + x + 1 = 0 — no rational root, D > 0 case (1 real + 2 complex)."""
    # Discriminant: -4(1)³ - 27(1)² = -4 - 27 = -31 < 0 (one real + 2 complex)
    a, b, c, d = _frac(1, 0, 1, 1)
    roots = solve_cubic(a, b, c, d)
    assert isinstance(roots, list)
    # Should return 3 roots (1 real Cbrt/expression + 2 complex)
    assert len(roots) == 3


def test_cubic_casus_irreducibilis_returns_empty() -> None:
    """x³ - 3x + 1 = 0 — three distinct real roots, no rational root → []."""
    # This has 3 real irrational roots (casus irreducibilis).
    # Discriminant > 0 → D_cardano < 0 → return []
    a, b, c, d = _frac(1, 0, -3, 1)
    roots = solve_cubic(a, b, c, d)
    # Should return empty list (casus irreducibilis, can't express in real radicals)
    assert roots == []


def test_cubic_cardano_symbolic_neg_q_half_nonzero() -> None:
    """x³ + x + 1 = 0 — Cardano path with symbolic Cbrt (neg_q_half ≠ 0)."""
    a, b, c, d = _frac(1, 0, 1, 1)
    roots = solve_cubic(a, b, c, d)
    assert isinstance(roots, list)
    assert len(roots) == 3
    # At least one root is an IRApply (the Cbrt expression)
    assert any(isinstance(r, IRApply) for r in roots)


# ---------------------------------------------------------------------------
# Private helper tests (for coverage of internal paths)
# ---------------------------------------------------------------------------


def test_find_rational_root_negative_root() -> None:
    """_find_rational_root correctly identifies a negative root."""
    from fractions import Fraction
    from cas_solve.cubic import _find_rational_root, _eval_cubic

    a, b, c, d = Fraction(1), Fraction(1), Fraction(-4), Fraction(-4)
    # x³ + x² - 4x - 4 = (x+1)(x-2)(x+2) → roots -1, 2, -2
    r = _find_rational_root(a, b, c, d)
    assert r is not None
    assert _eval_cubic(a, b, c, d, r) == 0


def test_cardano_repeated_triple_root() -> None:
    """_cardano_repeated with p=q=0 → single triple root at shift."""
    from fractions import Fraction
    from cas_solve.cubic import _cardano_repeated
    from symbolic_ir import IRInteger

    # p=0, q=0 means t³=0 → t=0, shift=2 → root at 2
    roots = _cardano_repeated(Fraction(0), Fraction(0), Fraction(2))
    assert len(roots) == 1
    assert roots[0] == IRInteger(2)


def test_cardano_repeated_rational_cbrt() -> None:
    """_cardano_repeated with exact rational cube root."""
    from fractions import Fraction
    from cas_solve.cubic import _cardano_repeated
    from symbolic_ir import IRInteger

    # p=-3, q=2 → D=0 → -q/2 = -1, cbrt(-1) = -1
    # t1 = 2*(-1) = -2, t2 = -(-1) = 1, shift=0
    roots = _cardano_repeated(Fraction(-3), Fraction(2), Fraction(0))
    assert len(roots) == 2
    int_roots = {r.value for r in roots if isinstance(r, IRInteger)}
    assert int_roots == {-2, 1}


def test_try_exact_cbrt_negative() -> None:
    """_try_exact_cbrt handles negative values correctly."""
    from fractions import Fraction
    from cas_solve.cubic import _try_exact_cbrt

    assert _try_exact_cbrt(Fraction(-8)) == Fraction(-2)
    assert _try_exact_cbrt(Fraction(0)) == Fraction(0)
    assert _try_exact_cbrt(Fraction(27)) == Fraction(3)
    assert _try_exact_cbrt(Fraction(2)) is None  # not a perfect cube


def test_try_exact_sqrt_fractions() -> None:
    """_try_exact_sqrt handles rational fractions."""
    from fractions import Fraction
    from cas_solve.cubic import _try_exact_sqrt

    assert _try_exact_sqrt(Fraction(4)) == Fraction(2)
    assert _try_exact_sqrt(Fraction(1, 4)) == Fraction(1, 2)
    assert _try_exact_sqrt(Fraction(2)) is None
    assert _try_exact_sqrt(Fraction(-1)) is None


def test_dedup_and_sort_removes_duplicates() -> None:
    """_dedup_and_sort removes duplicate IRNode entries."""
    from cas_solve.cubic import _dedup_and_sort
    from symbolic_ir import IRInteger

    roots = [IRInteger(1), IRInteger(2), IRInteger(1), IRInteger(3)]
    result = _dedup_and_sort(roots)
    assert len(result) == 3
    assert IRInteger(1) in result


def test_imag_term_unit() -> None:
    """_imag_term(1) returns I_UNIT directly."""
    from fractions import Fraction
    from cas_solve.cubic import _imag_term
    from symbolic_ir import IRSymbol

    result = _imag_term(Fraction(1))
    assert isinstance(result, IRSymbol)
    assert result.name == "%i"


def test_imag_term_neg_one() -> None:
    """_imag_term(-1) returns Neg(I_UNIT)."""
    from fractions import Fraction
    from cas_solve.cubic import _imag_term
    from symbolic_ir import IRApply

    result = _imag_term(Fraction(-1))
    assert isinstance(result, IRApply)
    assert result.head.name == "Neg"


def test_add_shift_zero() -> None:
    """_add_shift with zero shift returns the node unchanged."""
    from fractions import Fraction
    from cas_solve.cubic import _add_shift
    from symbolic_ir import IRInteger

    node = IRInteger(5)
    assert _add_shift(node, Fraction(0)) is node


def test_add_shift_negative() -> None:
    """_add_shift with negative shift builds a Sub node."""
    from fractions import Fraction
    from cas_solve.cubic import _add_shift
    from symbolic_ir import IRApply, IRInteger

    node = IRInteger(5)
    result = _add_shift(node, Fraction(-3))
    assert isinstance(result, IRApply)
    assert result.head.name == "Sub"
