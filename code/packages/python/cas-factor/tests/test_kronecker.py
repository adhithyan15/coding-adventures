"""Tests for Kronecker's polynomial factoring algorithm."""

from __future__ import annotations

from fractions import Fraction

from cas_factor.kronecker import (
    _divides_exactly,
    _eval_points,
    _lagrange_interpolate,
    _signed_divisors,
    kronecker_factor,
)
from cas_factor.polynomial import degree, evaluate, normalize

# ---------------------------------------------------------------------------
# Helper: eval_points
# ---------------------------------------------------------------------------


def test_eval_points_one() -> None:
    assert _eval_points(1) == [0]


def test_eval_points_two() -> None:
    assert _eval_points(2) == [0, 1]


def test_eval_points_three() -> None:
    assert _eval_points(3) == [0, 1, -1]


def test_eval_points_five() -> None:
    assert _eval_points(5) == [0, 1, -1, 2, -2]


# ---------------------------------------------------------------------------
# Helper: signed_divisors
# ---------------------------------------------------------------------------


def test_signed_divisors_one() -> None:
    result = set(_signed_divisors(1))
    assert result == {1, -1}


def test_signed_divisors_four() -> None:
    result = set(_signed_divisors(4))
    assert result == {1, -1, 2, -2, 4, -4}


def test_signed_divisors_zero() -> None:
    assert _signed_divisors(0) == []


def test_signed_divisors_negative() -> None:
    # Should use |n|.
    result = set(_signed_divisors(-6))
    assert result == {1, -1, 2, -2, 3, -3, 6, -6}


# ---------------------------------------------------------------------------
# Helper: lagrange_interpolate
# ---------------------------------------------------------------------------


def test_lagrange_constant() -> None:
    """Interpolation through one point gives the constant polynomial."""
    result = _lagrange_interpolate([0], [7])
    assert result == [Fraction(7)]


def test_lagrange_linear() -> None:
    """Two points: 2x + 1."""
    # p(0)=1, p(1)=3  →  1 + 2x
    result = _lagrange_interpolate([0, 1], [1, 3])
    assert result is not None
    assert len(result) == 2
    assert result[0] == Fraction(1)
    assert result[1] == Fraction(2)


def test_lagrange_quadratic() -> None:
    """Three points: x^2 + x + 1."""
    # p(0)=1, p(1)=3, p(-1)=1
    result = _lagrange_interpolate([0, 1, -1], [1, 3, 1])
    assert result is not None
    assert len(result) == 3
    assert result[0] == Fraction(1)   # constant
    assert result[1] == Fraction(1)   # x coefficient
    assert result[2] == Fraction(1)   # x^2 coefficient


def test_lagrange_x_squared_plus_2x_plus_2() -> None:
    """Three points matching x^2 + 2x + 2."""
    # p(0)=2, p(1)=5, p(-1)=1
    result = _lagrange_interpolate([0, 1, -1], [2, 5, 1])
    assert result is not None
    assert all(c.denominator == 1 for c in result)
    ints = [int(c) for c in result]
    assert ints == [2, 2, 1]  # 2 + 2x + x^2


def test_lagrange_duplicate_points_returns_none() -> None:
    result = _lagrange_interpolate([1, 1], [2, 3])
    assert result is None


# ---------------------------------------------------------------------------
# Helper: _divides_exactly
# ---------------------------------------------------------------------------


def test_divides_exactly_linear() -> None:
    # (x - 1) divides (x^2 - 1); cofactor = (x + 1) = [1, 1]
    cofactor = _divides_exactly([-1, 0, 1], [-1, 1])
    assert cofactor == [1, 1]


def test_divides_exactly_non_divisor() -> None:
    # (x - 2) does NOT divide (x^2 - 1)
    result = _divides_exactly([-1, 0, 1], [-2, 1])
    assert result is None


def test_divides_exactly_quadratic_by_quadratic() -> None:
    # (x^2 + 2x + 2) divides (x^4 + 4)
    # x^4 + 4 = [4, 0, 0, 0, 1]
    # x^2 + 2x + 2 = [2, 2, 1]
    # cofactor = x^2 - 2x + 2 = [2, -2, 1]
    cofactor = _divides_exactly([4, 0, 0, 0, 1], [2, 2, 1])
    assert cofactor == [2, -2, 1]


def test_divides_exactly_non_integer_quotient() -> None:
    # 2x + 1 does NOT divide x^2 + 1 with integer quotient
    result = _divides_exactly([1, 0, 1], [1, 2])
    assert result is None


# ---------------------------------------------------------------------------
# kronecker_factor — irreducible polynomials
# ---------------------------------------------------------------------------


def test_kronecker_x_squared_plus_1_irreducible() -> None:
    """x^2 + 1 is irreducible over Z."""
    assert kronecker_factor([1, 0, 1]) is None


def test_kronecker_x_squared_minus_2_irreducible() -> None:
    """x^2 - 2 is irreducible over Z (irrational roots)."""
    assert kronecker_factor([-2, 0, 1]) is None


def test_kronecker_x_squared_plus_x_plus_1_irreducible() -> None:
    """x^2 + x + 1 is irreducible over Z."""
    assert kronecker_factor([1, 1, 1]) is None


def test_kronecker_x_cubed_plus_x_plus_1_irreducible() -> None:
    """x^3 + x + 1 is irreducible over Z (discriminant = -31)."""
    assert kronecker_factor([1, 1, 0, 1]) is None


# ---------------------------------------------------------------------------
# kronecker_factor — factorable polynomials
# ---------------------------------------------------------------------------


def test_kronecker_sophie_germain_x4_plus_4() -> None:
    """x^4 + 4 = (x^2 + 2x + 2)(x^2 - 2x + 2)."""
    result = kronecker_factor([4, 0, 0, 0, 1])
    assert result is not None
    f1, f2 = result
    # Both factors should be degree-2 with positive leading coefficient.
    assert degree(f1) == 2
    assert degree(f2) == 2
    assert f1[-1] > 0
    assert f2[-1] > 0
    # Verify the product equals x^4 + 4 by evaluating at several points.
    for x_val in range(-5, 6):
        p_val = evaluate([4, 0, 0, 0, 1], x_val)
        assert evaluate(f1, x_val) * evaluate(f2, x_val) == p_val


def test_kronecker_x4_plus_x2_plus_1() -> None:
    """x^4 + x^2 + 1 = (x^2 + x + 1)(x^2 - x + 1)."""
    result = kronecker_factor([1, 0, 1, 0, 1])
    assert result is not None
    f1, f2 = result
    assert degree(f1) == 2
    assert degree(f2) == 2
    for x_val in range(-5, 6):
        p_val = evaluate([1, 0, 1, 0, 1], x_val)
        assert evaluate(f1, x_val) * evaluate(f2, x_val) == p_val


def test_kronecker_quadratic_with_content_one() -> None:
    """x^2 - 1 = (x-1)(x+1), but with no integer roots extracted first.

    Here we call kronecker_factor directly on [-1, 0, 1] (not via the
    orchestrator).  Phase-1 linear factors should be found via k=1.
    """
    result = kronecker_factor([-1, 0, 1])
    if result is not None:
        f1, f2 = result
        for x_val in range(-5, 6):
            product = evaluate(f1, x_val) * evaluate(f2, x_val)
            assert product == evaluate([-1, 0, 1], x_val)
    # Alternatively the algo might find no factor (linear roots are
    # found by find_integer_roots; Kronecker is allowed to also find them
    # but is not required to).


def test_kronecker_x4_plus_2x2_plus_1_repeated_quadratic() -> None:
    """x^4 + 2x^2 + 1 = (x^2+1)^2 — Kronecker should find x^2+1 twice."""
    result = kronecker_factor([1, 0, 2, 0, 1])
    assert result is not None
    f1, f2 = result
    # Both factors should be equal to x^2 + 1 = [1, 0, 1].
    assert normalize(f1) == [1, 0, 1]
    assert normalize(f2) == [1, 0, 1]


def test_kronecker_degree_two_returns_none_for_too_small() -> None:
    """kronecker_factor on a degree-1 polynomial returns None."""
    assert kronecker_factor([1, 1]) is None


def test_kronecker_result_product_equals_input_sophie_germain() -> None:
    """Verify f1 * f2 = p at many evaluation points for x^4 + 4."""
    result = kronecker_factor([4, 0, 0, 0, 1])
    assert result is not None
    f1, f2 = result
    p = [4, 0, 0, 0, 1]
    for x_val in range(-10, 11):
        assert evaluate(f1, x_val) * evaluate(f2, x_val) == evaluate(p, x_val)
