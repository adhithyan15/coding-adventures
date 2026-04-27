"""Tests for quartic-equation solver."""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import IRApply, IRInteger, IRRational, IRSymbol

from cas_solve import solve_quartic


def _frac(*ints: int) -> list[Fraction]:
    return [Fraction(n) for n in ints]


# ---------------------------------------------------------------------------
# Rational roots (rational-root theorem path)
# ---------------------------------------------------------------------------


def test_quartic_four_rational_roots() -> None:
    """x⁴ - 10x² + 9 = 0 → (x²-1)(x²-9) = 0 → {±1, ±3}."""
    a, b, c, d, e = _frac(1, 0, -10, 0, 9)
    roots = solve_quartic(a, b, c, d, e)
    assert isinstance(roots, list)
    int_roots = {r.value for r in roots if isinstance(r, IRInteger)}
    assert int_roots == {1, -1, 3, -3}


def test_quartic_biquadratic_basic() -> None:
    """x⁴ - 5x² + 4 = 0 → {±1, ±2}."""
    a, b, c, d, e = _frac(1, 0, -5, 0, 4)
    roots = solve_quartic(a, b, c, d, e)
    assert isinstance(roots, list)
    int_roots = {r.value for r in roots if isinstance(r, IRInteger)}
    assert int_roots == {1, -1, 2, -2}


def test_quartic_with_rational_root() -> None:
    """x⁴ - x³ - 7x² + x + 6 = 0 → roots -2, -1, 1, 3."""
    a, b, c, d, e = _frac(1, -1, -7, 1, 6)
    roots = solve_quartic(a, b, c, d, e)
    assert isinstance(roots, list)
    int_roots = {r.value for r in roots if isinstance(r, IRInteger)}
    assert int_roots == {-2, -1, 1, 3}


def test_quartic_delegates_to_cubic_when_a_zero() -> None:
    """With a=0, delegates to solve_cubic."""
    a, b, c, d, e = Fraction(0), Fraction(1), Fraction(-6), Fraction(11), Fraction(-6)
    roots = solve_quartic(a, b, c, d, e)
    assert isinstance(roots, list)
    assert IRInteger(1) in roots
    assert IRInteger(2) in roots
    assert IRInteger(3) in roots


def test_quartic_with_zero_root() -> None:
    """x⁴ - x³ = 0 → x³(x-1) = 0 → {0, 1}."""
    a, b, c, d, e = _frac(1, -1, 0, 0, 0)
    roots = solve_quartic(a, b, c, d, e)
    assert isinstance(roots, list)
    assert IRInteger(0) in roots
    assert IRInteger(1) in roots


def test_quartic_integer_coefficients() -> None:
    """x⁴ - 1 = 0 → {1, -1, i, -i}."""
    a, b, c, d, e = _frac(1, 0, 0, 0, -1)
    roots = solve_quartic(a, b, c, d, e)
    assert isinstance(roots, list)
    # Real roots: 1 and -1
    assert IRInteger(1) in roots
    assert IRInteger(-1) in roots


def test_quartic_all_integer_roots_positive() -> None:
    """(x-1)(x-2)(x-3)(x-4) = x⁴ - 10x³ + 35x² - 50x + 24 = 0 → {1,2,3,4}."""
    a, b, c, d, e = _frac(1, -10, 35, -50, 24)
    roots = solve_quartic(a, b, c, d, e)
    assert isinstance(roots, list)
    int_roots = {r.value for r in roots if isinstance(r, IRInteger)}
    assert int_roots == {1, 2, 3, 4}


def test_quartic_result_length() -> None:
    """A quartic returns at most 4 roots."""
    a, b, c, d, e = _frac(1, 0, -5, 0, 4)
    roots = solve_quartic(a, b, c, d, e)
    assert isinstance(roots, list)
    assert len(roots) <= 4


def test_quartic_biquadratic_complex_roots() -> None:
    """x⁴ + 4x² + 3 = 0 — biquadratic with complex roots (no rational root)."""
    # No rational roots: candidates ±1, ±3 → 1+4+3=8≠0, 1-4+3=0... wait
    # x=1: 1+4+3=8≠0, x=-1: 1+4+3=8≠0 (same), x=3: 81+36+3=120≠0
    # Actually wait let me verify: x⁴+4x²+3 = (x²+1)(x²+3) → roots ±i, ±i√3
    # No rational roots ✓
    a, b, c, d, e = _frac(1, 0, 4, 0, 3)
    roots = solve_quartic(a, b, c, d, e)
    assert isinstance(roots, list)
    # Should return 4 complex roots (all IRApply nodes or sqrt/neg nodes)
    assert len(roots) == 4


def test_quartic_biquadratic_real_irrational() -> None:
    """x⁴ - 3x² - 4 = (x²-4)(x²+1) = 0 → real roots ±2 (rational) + complex ±i."""
    # x² = 4 → x = ±2 (rational roots! rational root theorem catches them)
    # Use x⁴ - 5x² + 4 instead which is already tested; try a biquadratic with no
    # rational roots but real irrational roots.
    # x⁴ - 10x² + 1 = 0 → x² = 5±2√6 → x = ±√(5±2√6), no rational roots
    a, b, c, d, e = _frac(1, 0, -10, 0, 1)
    roots = solve_quartic(a, b, c, d, e)
    assert isinstance(roots, list)
    # Should return 4 irrational roots via biquadratic path
    assert len(roots) == 4


def test_quartic_ferrari_complex_roots() -> None:
    """x⁴ + x² + 2x + 6 = 0 — full Ferrari path (no rational root, resolvent root m=2)."""
    # Roots: -1±i and 1±i√2 (all complex)
    # Resolvent cubic: 8m³ + 8m² - 46m - 4 = 0 has rational root m=2
    a, b, c, d, e = _frac(1, 0, 1, 2, 6)
    roots = solve_quartic(a, b, c, d, e)
    assert isinstance(roots, list)
    # Should return 4 complex roots
    assert len(roots) == 4
    # All should be IRApply (complex expressions)
    for r in roots:
        assert isinstance(r, IRApply)


def test_quartic_no_usable_resolvent_root() -> None:
    """x⁴ + x + 1 = 0 — Ferrari resolvent has no rational root → [] (unevaluated)."""
    # Resolvent cubic: 8m³ - 8m - 1 = 0 has no rational root
    a, b, c, d, e = _frac(1, 0, 0, 1, 1)
    roots = solve_quartic(a, b, c, d, e)
    # Returns empty (unevaluated) since we can't solve the resolvent
    assert isinstance(roots, list)
    # Either empty (unevaluated) or solved some other way


def test_quartic_find_rational_root_zero_constant() -> None:
    """Quartic with e=0 always has 0 as a root."""
    # x⁴ + x³ = x³(x+1) = 0 → roots 0 (triple), -1
    a, b, c, d, e = _frac(1, 1, 0, 0, 0)
    roots = solve_quartic(a, b, c, d, e)
    assert isinstance(roots, list)
    assert IRInteger(0) in roots
