"""solve_quadratic tests."""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import IRInteger

from cas_solve import solve_quadratic


def test_two_distinct_real_roots() -> None:
    """x^2 - 5x + 6 = 0 → x in {2, 3}."""
    out = solve_quadratic(Fraction(1), Fraction(-5), Fraction(6))
    assert out == [IRInteger(2), IRInteger(3)]


def test_double_root() -> None:
    """x^2 - 4x + 4 = 0 → x = 2 (single repeated root)."""
    out = solve_quadratic(Fraction(1), Fraction(-4), Fraction(4))
    assert out == [IRInteger(2)]


def test_complex_roots() -> None:
    """x^2 + 1 = 0 → x in {i, -i}."""
    out = solve_quadratic(Fraction(1), Fraction(0), Fraction(1))
    # Both elements should reference %i; we just check the count and
    # that %i appears in the structure.
    assert isinstance(out, list)
    assert len(out) == 2
    text = repr(out)
    assert "%i" in text


def test_zero_leading_falls_back_to_linear() -> None:
    """0*x^2 + 2x + 4 = 0 → x = -2."""
    out = solve_quadratic(Fraction(0), Fraction(2), Fraction(4))
    assert out == [IRInteger(-2)]


def test_irrational_discriminant() -> None:
    """x^2 - 2 = 0 → roots involving sqrt(2)."""
    out = solve_quadratic(Fraction(1), Fraction(0), Fraction(-2))
    assert isinstance(out, list)
    assert len(out) == 2
    text = repr(out)
    assert "Sqrt" in text


def test_perfect_square_discriminant_with_rational_coeffs() -> None:
    """(2x - 1)(2x + 1) = 4x^2 - 1 → x in {1/2, -1/2}."""
    out = solve_quadratic(Fraction(4), Fraction(0), Fraction(-1))
    from symbolic_ir import IRRational

    assert IRRational(-1, 2) in out
    assert IRRational(1, 2) in out
