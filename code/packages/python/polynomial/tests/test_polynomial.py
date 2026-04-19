"""Tests for the polynomial package."""

from fractions import Fraction

import pytest
from polynomial import (
    VERSION,
    add,
    degree,
    deriv,
    divide,
    divmod_poly,
    evaluate,
    gcd,
    mod,
    monic,
    multiply,
    normalize,
    one,
    squarefree,
    subtract,
    zero,
)


# =============================================================================
# Helpers
# =============================================================================


def poly_equal(a, b, eps=1e-9):
    """Compare two polynomials coefficient-by-coefficient within epsilon."""
    if len(a) != len(b):
        return False
    return all(abs(x - y) <= eps for x, y in zip(a, b))


# =============================================================================
# VERSION
# =============================================================================


class TestVersion:
    def test_is_semver(self):
        parts = VERSION.split(".")
        assert len(parts) == 3
        assert all(p.isdigit() for p in parts)


# =============================================================================
# normalize
# =============================================================================


class TestNormalize:
    def test_strips_trailing_zeros(self):
        assert normalize((1, 0, 0)) == (1,)

    def test_all_zeros_becomes_empty(self):
        assert normalize((0,)) == ()
        assert normalize((0, 0, 0)) == ()

    def test_empty_is_empty(self):
        assert normalize(()) == ()

    def test_already_normalized_unchanged(self):
        assert normalize((1, 2, 3)) == (1, 2, 3)

    def test_preserves_internal_zeros(self):
        assert normalize((1, 0, 2)) == (1, 0, 2)

    def test_single_nonzero(self):
        assert normalize((5,)) == (5,)


# =============================================================================
# degree
# =============================================================================


class TestDegree:
    def test_zero_polynomial_is_minus_one(self):
        assert degree(()) == -1
        assert degree((0,)) == -1
        assert degree((0, 0, 0)) == -1

    def test_constant_polynomial_is_degree_zero(self):
        assert degree((7,)) == 0
        assert degree((1,)) == 0

    def test_linear_polynomial(self):
        assert degree((1, 2)) == 1

    def test_quadratic_polynomial(self):
        assert degree((1, 2, 3)) == 2
        assert degree((3, 0, 2)) == 2

    def test_ignores_trailing_zeros(self):
        assert degree((3, 0, 0)) == 0
        assert degree((1, 2, 0)) == 1


# =============================================================================
# zero and one
# =============================================================================


class TestZeroAndOne:
    def test_zero_is_empty(self):
        assert zero() == ()

    def test_zero_is_additive_identity(self):
        p = (1, 2, 3)
        assert add(zero(), p) == p
        assert add(p, zero()) == p

    def test_one_is_singleton(self):
        assert one() == (1,)

    def test_one_is_multiplicative_identity(self):
        p = (1, 2, 3)
        assert multiply(one(), p) == p
        assert multiply(p, one()) == p


# =============================================================================
# add
# =============================================================================


class TestAdd:
    def test_same_length(self):
        assert add((1, 2, 3), (4, 5, 6)) == (5, 7, 9)

    def test_shorter_first(self):
        assert add((4, 5), (1, 2, 3)) == (5, 7, 3)

    def test_shorter_second(self):
        assert add((1, 2, 3), (4, 5)) == (5, 7, 3)

    def test_cancellation_gives_zero(self):
        assert add((1, 2, 3), (-1, -2, -3)) == ()

    def test_zero_is_identity(self):
        assert add((), (1, 2)) == (1, 2)
        assert add((1, 2), ()) == (1, 2)

    def test_commutative(self):
        a, b = (1, 2, 3), (4, 5, 6, 7)
        assert add(a, b) == add(b, a)

    def test_normalizes_result(self):
        assert add((1, 2, 3), (0, 0, -3)) == (1, 2)


# =============================================================================
# subtract
# =============================================================================


class TestSubtract:
    def test_basic(self):
        # [5,7,3] - [1,2,3] = [4,5,0] → (4,5)
        assert subtract((5, 7, 3), (1, 2, 3)) == (4, 5)

    def test_self_minus_self_is_zero(self):
        assert subtract((1, 2, 3), (1, 2, 3)) == ()

    def test_zero_is_identity(self):
        assert subtract((1, 2, 3), ()) == (1, 2, 3)

    def test_round_trip(self):
        p = (3, 1, 4)
        q = (1, 5, 9)
        assert add(subtract(p, q), q) == p

    def test_shorter_second(self):
        result = subtract((1, 2, 3), (1, 2))
        assert result == (0, 0, 3)


# =============================================================================
# multiply
# =============================================================================


class TestMultiply:
    def test_two_linear(self):
        # (1+2x)(3+4x) = 3 + 10x + 8x²
        assert multiply((1, 2), (3, 4)) == (3, 10, 8)

    def test_multiply_by_zero(self):
        assert multiply((1, 2, 3), ()) == ()
        assert multiply((), (1, 2, 3)) == ()

    def test_multiply_by_one(self):
        assert multiply((1, 2, 3), (1,)) == (1, 2, 3)
        assert multiply((1,), (1, 2, 3)) == (1, 2, 3)

    def test_commutative(self):
        a, b = (1, 2, 3), (4, 5)
        ab = multiply(a, b)
        ba = multiply(b, a)
        assert poly_equal(ab, ba)

    def test_associative(self):
        a, b, c = (1, 2), (3, 4), (5, 6)
        left = multiply(multiply(a, b), c)
        right = multiply(a, multiply(b, c))
        assert poly_equal(left, right)

    def test_distributive_over_add(self):
        a, b, c = (1, 2), (3, 4), (5, 6)
        lhs = multiply(a, add(b, c))
        rhs = add(multiply(a, b), multiply(a, c))
        assert poly_equal(lhs, rhs)

    def test_result_degree_is_sum(self):
        a = (1, 2, 3)  # degree 2
        b = (4, 5, 6)  # degree 2
        result = multiply(a, b)
        assert degree(result) == 4  # degree 4

    def test_constant_times_constant(self):
        assert multiply((3,), (4,)) == (12.0,)


# =============================================================================
# divmod_poly
# =============================================================================


class TestDivmodPoly:
    def test_raises_for_zero_divisor(self):
        with pytest.raises(ValueError):
            divmod_poly((1, 2, 3), ())
        with pytest.raises(ValueError):
            divmod_poly((1, 2, 3), (0,))

    def test_low_degree_dividend(self):
        q, r = divmod_poly((1, 2), (0, 0, 1))
        assert q == ()
        assert r == (1, 2)

    def test_zero_remainder(self):
        product = multiply((1, 1), (1, 1))  # (1+x)^2 = 1 + 2x + x²
        q, r = divmod_poly(product, (1, 1))
        assert poly_equal(q, (1, 1))
        assert r == ()

    def test_a_equals_b_times_q_plus_r(self):
        a = (5, 1, 3, 2)
        b = (2, 1)
        q, r = divmod_poly(a, b)
        reconstructed = add(multiply(b, q), r)
        assert poly_equal(reconstructed, normalize(a))

    def test_spec_example(self):
        # Detailed example from spec: (5 + x + 3x² + 2x³) / (2 + x)
        # Quotient: 3 - x + 2x²; Remainder: -1
        a = (5, 1, 3, 2)
        b = (2, 1)
        q, r = divmod_poly(a, b)
        assert poly_equal(q, (3, -1, 2))
        assert poly_equal(r, (-1,))

    def test_constant_divisor(self):
        q, r = divmod_poly((4, 6, 8), (2,))
        assert poly_equal(q, (2, 3, 4))
        assert r == ()

    def test_divides_itself(self):
        p = (1, 2, 3)
        q, r = divmod_poly(p, p)
        assert poly_equal(q, (1,))
        assert r == ()


# =============================================================================
# divide and mod
# =============================================================================


class TestDivide:
    def test_basic(self):
        # 1 - x² = (1-x)(1+x); divide by (1+x) should give (-1+x) or (1-x)
        a = (1, 0, -1)
        b = (1, 1)
        q = divide(a, b)
        # verify b*q = a
        assert poly_equal(multiply(b, q), normalize(a))

    def test_raises_for_zero_divisor(self):
        with pytest.raises(ValueError):
            divide((1, 2), ())


class TestMod:
    def test_returns_remainder(self):
        a = (1, 2, 3)
        b = (1, 1)
        r = mod(a, b)
        q = divide(a, b)
        assert poly_equal(add(multiply(b, q), r), normalize(a))

    def test_exact_division_is_zero(self):
        p = multiply((1, 1), (2, 1))
        assert mod(p, (1, 1)) == ()

    def test_raises_for_zero_divisor(self):
        with pytest.raises(ValueError):
            mod((1, 2), ())


# =============================================================================
# evaluate
# =============================================================================


class TestEvaluate:
    def test_zero_polynomial(self):
        assert evaluate((), 5) == 0
        assert evaluate((), 0) == 0

    def test_constant_polynomial(self):
        assert evaluate((7,), 0) == 7
        assert evaluate((7,), 100) == 7

    def test_linear_at_zero(self):
        assert evaluate((3, 2), 0) == 3

    def test_linear_at_one(self):
        # 3 + 2·1 = 5
        assert evaluate((3, 2), 1) == 5

    def test_quadratic_spec_example(self):
        # 3 + x + 2x² at x=4 → 39
        assert evaluate((3, 1, 2), 4) == pytest.approx(39.0)

    def test_at_zero_is_constant_term(self):
        assert evaluate((5, 3, 1), 0) == 5

    def test_matches_naive_evaluation(self):
        p = (1, -3, 2)  # 1 - 3x + 2x²
        x = 3
        naive = p[0] + p[1] * x + p[2] * x * x
        assert evaluate(p, x) == pytest.approx(naive)

    def test_horner_consistency(self):
        """evaluate agrees with sum(coeff * x**i) naive computation."""
        p = (2, 0, -1, 3)
        x = 2.5
        naive = sum(c * x**i for i, c in enumerate(p))
        assert evaluate(p, x) == pytest.approx(naive, rel=1e-9)


# =============================================================================
# gcd
# =============================================================================


class TestGcd:
    def test_gcd_with_zero_returns_other(self):
        p = (1, 2, 3)
        assert poly_equal(gcd(p, ()), normalize(p))
        assert poly_equal(gcd((), p), normalize(p))

    def test_gcd_of_self(self):
        p = (1, 2, 3)
        g = gcd(p, p)
        assert degree(g) == degree(p)

    def test_coprime_polynomials_have_constant_gcd(self):
        a = (1, 1)  # 1 + x
        b = (2, 1)  # 2 + x
        g = gcd(a, b)
        assert degree(g) == 0

    def test_common_factor_detected(self):
        # (x+1)(x+2) and (x+1)(x+3): GCD should have degree 1
        f1 = multiply((1, 1), (2, 1))
        f2 = multiply((1, 1), (3, 1))
        g = gcd(f1, f2)
        assert degree(g) == 1
        # g must divide both f1 and f2 exactly
        assert mod(f1, g) == ()
        assert mod(f2, g) == ()


# =============================================================================
# Exact arithmetic with Fraction coefficients — the CAS path
# =============================================================================


class TestFractionWorkflow:
    """``Fraction`` coefficients must round-trip exactly through every
    arithmetic op. A single ``0.0`` accumulator would silently demote to
    ``float`` and break exactness — hence these checks.
    """

    def test_multiply_preserves_fraction(self):
        a = (Fraction(1, 2), Fraction(1, 3))
        b = (Fraction(1, 5),)
        result = multiply(a, b)
        # (1/2 + x/3) · (1/5) = 1/10 + x/15
        assert result == (Fraction(1, 10), Fraction(1, 15))
        assert all(isinstance(c, Fraction) for c in result)

    def test_divmod_preserves_fraction(self):
        # (1/2 + x) ÷ (1/3) = (3/2 + 3x) with remainder 0.
        a = (Fraction(1, 2), Fraction(1))
        b = (Fraction(1, 3),)
        q, r = divmod_poly(a, b)
        assert q == (Fraction(3, 2), Fraction(3))
        assert r == ()

    def test_evaluate_preserves_fraction(self):
        p = (Fraction(1, 2), Fraction(1, 3))
        # (1/2 + x/3) at x = 6 = 1/2 + 2 = 5/2
        assert evaluate(p, Fraction(6)) == Fraction(5, 2)

    def test_gcd_over_Q(self):
        # (x - 1/2)·(x - 2)  and  (x - 1/2)·(x - 3)
        common = (Fraction(-1, 2), Fraction(1))
        f1 = multiply(common, (Fraction(-2), Fraction(1)))
        f2 = multiply(common, (Fraction(-3), Fraction(1)))
        g = gcd(f1, f2)
        assert degree(g) == 1


# =============================================================================
# deriv
# =============================================================================


class TestDeriv:
    def test_zero_polynomial(self):
        assert deriv(()) == ()

    def test_constant(self):
        assert deriv((7,)) == ()

    def test_linear(self):
        # d/dx (3 + x) = 1
        assert deriv((3, 1)) == (1,)

    def test_quadratic(self):
        # d/dx (3 + x + 2x²) = 1 + 4x
        assert deriv((3, 1, 2)) == (1, 4)

    def test_cubic(self):
        # d/dx (5x³) = 15x²
        assert deriv((0, 0, 0, 5)) == (0, 0, 15)

    def test_fraction_coefficients(self):
        # d/dx (1/2 + x/3 + x²/6) = 1/3 + x/3
        p = (Fraction(1, 2), Fraction(1, 3), Fraction(1, 6))
        assert deriv(p) == (Fraction(1, 3), Fraction(1, 3))

    def test_derivative_is_linear(self):
        # deriv(a + b) == deriv(a) + deriv(b)
        a = (1, 2, 3)
        b = (4, 5, 6, 7)
        assert deriv(add(a, b)) == add(deriv(a), deriv(b))

    def test_product_rule(self):
        # deriv(a·b) == deriv(a)·b + a·deriv(b)
        a = (1, 1)
        b = (2, 3, 1)
        lhs = deriv(multiply(a, b))
        rhs = add(multiply(deriv(a), b), multiply(a, deriv(b)))
        assert lhs == rhs


# =============================================================================
# monic
# =============================================================================


class TestMonic:
    def test_zero_polynomial(self):
        assert monic(()) == ()

    def test_already_monic(self):
        p = (Fraction(3), Fraction(2), Fraction(1))
        assert monic(p) == p

    def test_rescales_by_leading(self):
        # 2 + 4x + 6x²  →  1/3 + 2x/3 + x²
        p = (Fraction(2), Fraction(4), Fraction(6))
        assert monic(p) == (
            Fraction(1, 3),
            Fraction(2, 3),
            Fraction(1),
        )

    def test_leading_is_one(self):
        p = (Fraction(5), Fraction(10), Fraction(7))
        result = monic(p)
        assert result[-1] == Fraction(1)

    def test_constant_normalizes_to_one(self):
        assert monic((Fraction(7),)) == (Fraction(1),)


# =============================================================================
# squarefree — Yun's algorithm
# =============================================================================


def _expand_factors(factors, leading=Fraction(1)):
    """Reconstruct  c · s_1 · s_2^2 · … · s_k^k  from the factor list."""
    result = (leading,)
    for k, s in enumerate(factors, start=1):
        power = s
        for _ in range(k - 1):
            power = multiply(power, s)
        result = multiply(result, power)
    return result


class TestSquarefree:
    def test_zero_polynomial(self):
        assert squarefree(()) == []

    def test_constant(self):
        assert squarefree((Fraction(5),)) == []

    def test_squarefree_input_unchanged(self):
        # p = (x - 1)(x - 2) is squarefree; should return [p_monic].
        p = multiply(
            (Fraction(-1), Fraction(1)),
            (Fraction(-2), Fraction(1)),
        )
        assert squarefree(p) == [p]

    def test_linear_factor(self):
        # (x - 1) — already monic and squarefree.
        p = (Fraction(-1), Fraction(1))
        assert squarefree(p) == [p]

    def test_pure_square(self):
        # (x - 2)² → [1, x - 2]  (multiplicity 2 means position 2)
        x_minus_2 = (Fraction(-2), Fraction(1))
        p = multiply(x_minus_2, x_minus_2)
        factors = squarefree(p)
        assert len(factors) == 2
        assert degree(factors[0]) == 0      # no simple roots
        assert factors[1] == x_minus_2

    def test_mixed_multiplicity(self):
        # (x - 1)·(x - 2)²·(x - 3)³ → [x-1, x-2, x-3]
        x1 = (Fraction(-1), Fraction(1))
        x2 = (Fraction(-2), Fraction(1))
        x3 = (Fraction(-3), Fraction(1))
        p = multiply(x1, multiply(multiply(x2, x2), multiply(x3, multiply(x3, x3))))
        factors = squarefree(p)
        assert factors == [x1, x2, x3]

    def test_reconstruction(self):
        # squarefree(p) reconstructs to p (up to monic rescale).
        x1 = (Fraction(-1), Fraction(1))
        x2 = (Fraction(-2), Fraction(1))
        p = multiply(multiply(x1, x1), x2)  # (x-1)²(x-2)
        factors = squarefree(p)
        reconstructed = _expand_factors(factors, leading=Fraction(1))
        assert reconstructed == p

    def test_factors_are_monic(self):
        x1 = (Fraction(-1), Fraction(1))
        x2 = (Fraction(-2), Fraction(1))
        p = multiply(multiply(x1, x2), x2)
        factors = squarefree(p)
        for s in factors:
            if degree(s) > 0:
                assert s[-1] == Fraction(1)

    def test_factors_are_pairwise_coprime(self):
        x1 = (Fraction(-1), Fraction(1))
        x2 = (Fraction(-2), Fraction(1))
        x3 = (Fraction(-3), Fraction(1))
        p = multiply(x1, multiply(multiply(x2, x2), multiply(x3, multiply(x3, x3))))
        factors = squarefree(p)
        for i in range(len(factors)):
            for j in range(i + 1, len(factors)):
                if degree(factors[i]) > 0 and degree(factors[j]) > 0:
                    g = gcd(factors[i], factors[j])
                    assert degree(g) == 0
