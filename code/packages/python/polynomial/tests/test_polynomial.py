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
    extended_gcd,
    gcd,
    mod,
    monic,
    multiply,
    normalize,
    one,
    rational_roots,
    resultant,
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


# =============================================================================
# Extended Euclidean — the Bézout cofactors Hermite reduction needs
# =============================================================================


class TestExtendedGcd:
    """``extended_gcd`` returns ``(g, s, t)`` with ``s·a + t·b = g``.

    The identity is the one correctness gate — every test re-checks it.
    """

    def _bezout(self, a, b):
        g, s, t = extended_gcd(a, b)
        return g, s, t, add(multiply(s, a), multiply(t, b))

    def test_coprime_linears_return_constant_gcd(self):
        one_ = Fraction(1)
        a = (Fraction(-1), one_)  # x - 1
        b = (Fraction(1), one_)   # x + 1
        g, s, t, lhs = self._bezout(a, b)
        assert degree(g) == 0
        assert lhs == g

    def test_common_factor_is_recovered(self):
        one_ = Fraction(1)
        # (x - 1)(x + 2)  and  (x - 1)(x + 3): common factor (x - 1).
        f1 = multiply((Fraction(-1), one_), (Fraction(2), one_))
        f2 = multiply((Fraction(-1), one_), (Fraction(3), one_))
        g, s, t, lhs = self._bezout(f1, f2)
        assert lhs == g
        # g must divide both f1 and f2 exactly.
        assert mod(f1, g) == ()
        assert mod(f2, g) == ()

    def test_b_is_zero_returns_a_with_s_one(self):
        a = (Fraction(2), Fraction(3), Fraction(5))
        g, s, t = extended_gcd(a, ())
        assert g == a
        # s·a + t·0 = a  forces s = 1, t irrelevant.
        assert add(multiply(s, a), multiply(t, ())) == a

    def test_a_is_zero_returns_b_with_t_one(self):
        b = (Fraction(7), Fraction(0), Fraction(11))
        g, s, t = extended_gcd((), b)
        assert g == b
        assert add(multiply(s, ()), multiply(t, b)) == b

    def test_both_zero_returns_all_zero(self):
        g, s, t = extended_gcd((), ())
        assert g == ()

    def test_self_gcd_returns_self(self):
        p = (Fraction(1), Fraction(2), Fraction(3))
        g, s, t, lhs = self._bezout(p, p)
        assert degree(g) == degree(p)
        assert lhs == g

    def test_cofactor_identity_on_higher_degrees(self):
        one_ = Fraction(1)
        # (x - 1)² (x + 2) vs (x - 1) (x + 2)²: common (x - 1)(x + 2).
        xm1 = (Fraction(-1), one_)
        xp2 = (Fraction(2), one_)
        f1 = multiply(multiply(xm1, xm1), xp2)
        f2 = multiply(xm1, multiply(xp2, xp2))
        g, s, t, lhs = self._bezout(f1, f2)
        assert lhs == g
        assert degree(g) == 2

    def test_preserves_fraction_coefficients(self):
        # Force the divisions inside divmod to produce non-integer
        # rationals. The Bézout identity must still hold, and any
        # cofactor coefficient that actually passed through polynomial
        # division must be Fraction-typed (the integer seeds ``(1,)``
        # and ``()`` stay as-is when ``b`` divides ``a`` directly, but
        # any non-trivial Euclidean step introduces Fractions).
        a = (Fraction(1, 3), Fraction(2, 5), Fraction(1))
        b = (Fraction(1, 7), Fraction(1))
        g, s, t, lhs = self._bezout(a, b)
        assert lhs == g
        # g coefficients went through ``divmod_poly`` at least once.
        for c in g:
            assert isinstance(c, Fraction)


# =============================================================================
# Resultant
# =============================================================================


class TestResultant:
    """The resultant vanishes iff the two polynomials share a root."""

    def test_constant_second_arg(self):
        # res(p, c) = c^deg(p).
        p = (Fraction(1), Fraction(2), Fraction(3))  # degree 2
        c = (Fraction(5),)
        assert resultant(p, c) == Fraction(25)

    def test_constant_first_arg(self):
        # res(c, q) = c^deg(q) (up to the usual sign from the swap).
        c = (Fraction(7),)
        q = (Fraction(1), Fraction(2), Fraction(1))  # degree 2
        # The swap flips sign by (-1)^(0*2) = 1, so still 7^2 = 49.
        assert resultant(c, q) == Fraction(49)

    def test_coprime_linears(self):
        # res(x - 1, x - 2). Evaluating a at the root 2 of b gives 1,
        # times lc(b)^deg(a) = 1: magnitude 1. Sign depends on the
        # implementation; either way |res| = 1.
        xm1 = (Fraction(-1), Fraction(1))
        xm2 = (Fraction(-2), Fraction(1))
        r = resultant(xm1, xm2)
        assert abs(r) == 1

    def test_shared_root_vanishes(self):
        # (x - 1)(x + 2) and (x - 1)(x + 3) share root 1 ⇒ res = 0.
        xm1 = (Fraction(-1), Fraction(1))
        xp2 = (Fraction(2), Fraction(1))
        xp3 = (Fraction(3), Fraction(1))
        a = multiply(xm1, xp2)
        b = multiply(xm1, xp3)
        assert resultant(a, b) == 0

    def test_zero_polynomial_is_zero(self):
        assert resultant((), (Fraction(1), Fraction(1))) == 0
        assert resultant((Fraction(1), Fraction(1)), ()) == 0

    def test_product_identity(self):
        # res(a, b·c) == res(a, b) · res(a, c). Verifies multiplicativity
        # without depending on any particular sign convention.
        a = (Fraction(-2), Fraction(0), Fraction(1))  # x^2 - 2
        b = (Fraction(-3), Fraction(1))               # x - 3
        c = (Fraction(1), Fraction(1))                # x + 1
        r_bc = resultant(a, multiply(b, c))
        r_b = resultant(a, b)
        r_c = resultant(a, c)
        assert r_bc == r_b * r_c

    def test_swap_sign_flip_odd_degrees(self):
        # deg a < deg b ⇒ the implementation swaps inputs to keep the
        # Euclidean recurrence well-founded, and picks up the standard
        # (−1)^(deg a · deg b) sign. Exercise the odd·odd case with
        # deg a = 1, deg b = 3 so the swap actually fires (unlike
        # equal-degree calls, which don't trigger the swap).
        a = (Fraction(-1), Fraction(1))                           # x - 1
        b = (Fraction(-6), Fraction(11), Fraction(-6), Fraction(1))  # (x-1)(x-2)(x-3)
        # a shares root 1 with b ⇒ both directions give 0.
        assert resultant(a, b) == 0
        # Use coprime pair: a = x - 5, b = (x-1)(x-2)(x-3). Both
        # directions should be equal up to sign flip (-1)^(1·3) = -1.
        a = (Fraction(-5), Fraction(1))
        assert resultant(a, b) == -resultant(b, a)

    def test_evaluation_identity(self):
        # res(a, b) == lc(b)^deg(a) · ∏ a(β_j) where β_j are roots of b.
        # Pick b = (x - 1)(x - 2) with known roots.
        xm1 = (Fraction(-1), Fraction(1))
        xm2 = (Fraction(-2), Fraction(1))
        b = multiply(xm1, xm2)
        a = (Fraction(3), Fraction(0), Fraction(1))  # x^2 + 3
        # a(1) = 4, a(2) = 7. lc(b) = 1, deg(a) = 2.
        expected = Fraction(1) ** 2 * evaluate(a, Fraction(1)) * evaluate(a, Fraction(2))
        assert resultant(a, b) == expected


# =============================================================================
# Rational root finding
# =============================================================================


class TestRationalRoots:
    """Rational Roots Theorem on Q[z]."""

    def test_product_of_linears_integer_roots(self):
        # (z - 1)(z - 2)(z - 3).
        zm1 = (Fraction(-1), Fraction(1))
        zm2 = (Fraction(-2), Fraction(1))
        zm3 = (Fraction(-3), Fraction(1))
        p = multiply(multiply(zm1, zm2), zm3)
        assert rational_roots(p) == [Fraction(1), Fraction(2), Fraction(3)]

    def test_no_rational_roots(self):
        # z^2 - 2.
        p = (Fraction(-2), Fraction(0), Fraction(1))
        assert rational_roots(p) == []

    def test_fractional_roots_via_denominator(self):
        # (2z - 1)(3z + 2) = 6z^2 + z - 2.
        p = (Fraction(-2), Fraction(1), Fraction(6))
        assert rational_roots(p) == [Fraction(-2, 3), Fraction(1, 2)]

    def test_multiplicity_collapsed(self):
        # (z - 1)^3 has only one distinct root.
        zm1 = (Fraction(-1), Fraction(1))
        p = multiply(multiply(zm1, zm1), zm1)
        assert rational_roots(p) == [Fraction(1)]

    def test_zero_is_a_root(self):
        # z(z - 1) = z^2 - z. Constant term vanishes ⇒ 0 is a root.
        p = (Fraction(0), Fraction(-1), Fraction(1))
        assert rational_roots(p) == [Fraction(0), Fraction(1)]

    def test_constant_polynomial(self):
        assert rational_roots((Fraction(7),)) == []

    def test_zero_polynomial(self):
        assert rational_roots(()) == []

    def test_integer_coefficients(self):
        # Mixed types should still work via coercion.
        # z^2 - 1 with plain ints.
        assert rational_roots((-1, 0, 1)) == [Fraction(-1), Fraction(1)]

    def test_fraction_coefficients_in_input(self):
        # (1/2)z - 1 — root is z = 2.
        p = (Fraction(-1), Fraction(1, 2))
        assert rational_roots(p) == [Fraction(2)]

    def test_linear_polynomial(self):
        # A single linear factor.
        p = (Fraction(-5), Fraction(2))  # 2z - 5 ⇒ z = 5/2
        assert rational_roots(p) == [Fraction(5, 2)]

    def test_negative_leading_coefficient(self):
        # -z + 1 = 0 ⇒ z = 1. Exercises the sign-normalise path.
        p = (Fraction(1), Fraction(-1))
        assert rational_roots(p) == [Fraction(1)]
