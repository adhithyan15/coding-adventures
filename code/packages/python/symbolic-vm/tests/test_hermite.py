"""Tests for the Hermite reduction core.

The correctness gate is the re-differentiation identity: for any input
``num/den``, we must have

    d/dx (rat_num / rat_den)  +  log_num / log_den  ==  num / den.

Every test ends by checking that identity — a purely polynomial check
that doesn't depend on symbolic simplification. If it holds, Hermite
has produced a valid decomposition; if it doesn't, there's a bug.
"""

from __future__ import annotations

from fractions import Fraction

import pytest
from polynomial import (
    add,
    deriv,
    gcd,
    monic,
    multiply,
    normalize,
    subtract,
)

from symbolic_vm.hermite import hermite_reduce

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def P(*coefs):
    """Build a polynomial with Fraction coefficients from its coefs."""
    return tuple(Fraction(c) for c in coefs)


def rat_equal(a, b):
    """Compare two rational functions ``(n_a, d_a) == (n_b, d_b)`` as values.

    Use cross-multiplication so differing-but-equivalent representations
    (e.g., both sides scaled by the same constant) compare equal.
    """
    na, da = a
    nb, db = b
    lhs = multiply(na, db)
    rhs = multiply(nb, da)
    return normalize(lhs) == normalize(rhs)


def deriv_of_rational(num, den):
    """Return (num', den') such that (num'/den') == d/dx(num/den).

    Quotient rule: (u/v)' = (u'v - uv') / v². Every output stays in Q[x].
    """
    if not normalize(num):
        return ((), (Fraction(1),))
    u_prime = deriv(num)
    v_prime = deriv(den)
    new_num = subtract(multiply(u_prime, den), multiply(num, v_prime))
    new_den = multiply(den, den)
    return (new_num, new_den)


def rational_add(a, b):
    """Field-of-fractions add."""
    return (
        add(multiply(a[0], b[1]), multiply(b[0], a[1])),
        multiply(a[1], b[1]),
    )


def check_hermite_identity(num, den):
    """Run Hermite and verify the re-differentiation identity."""
    (rat, log) = hermite_reduce(num, den)
    # d/dx(rat) + log must equal num/den.
    rat_deriv = deriv_of_rational(*rat)
    total = rational_add(rat_deriv, log)
    assert rat_equal(total, (num, den)), (
        f"Hermite identity failed:\n"
        f"  input = {num}/{den}\n"
        f"  rat = {rat[0]}/{rat[1]}\n"
        f"  log = {log[0]}/{log[1]}\n"
        f"  d/dx(rat) + log = {total[0]}/{total[1]}"
    )
    return (rat, log)


def assert_squarefree(p):
    """The log-part denominator is required to be squarefree."""
    if len(p) <= 1:
        return
    g = gcd(p, deriv(p))
    assert (len(normalize(g)) <= 1) or (len(monic(g)) == 1)


# ---------------------------------------------------------------------------
# Cases with an exclusively rational antiderivative
# ---------------------------------------------------------------------------


class TestPureRational:
    """Integrands whose antiderivative is itself a rational function —
    the log part must come out zero."""

    def test_one_over_linear_squared(self):
        # ∫ 1/(x - 1)^2 dx = -1/(x - 1).
        #   num = 1, den = (x - 1)² = (1, -2, 1).
        num = P(1)
        xm1 = P(-1, 1)
        den = multiply(xm1, xm1)
        (rat, log) = check_hermite_identity(num, den)
        assert_squarefree(log[1])
        assert normalize(log[0]) == ()
        # rat == -1/(x-1).
        assert rat_equal(rat, (P(-1), xm1))

    def test_one_over_linear_cubed(self):
        # ∫ 1/(x - 1)^3 dx = -1/(2 (x - 1)²).
        xm1 = P(-1, 1)
        den = multiply(xm1, multiply(xm1, xm1))
        (rat, log) = check_hermite_identity(P(1), den)
        assert normalize(log[0]) == ()

    def test_one_over_fourth_power(self):
        # ∫ 1/(x - 1)^4 dx = -1/(3 (x - 1)^3).
        xm1 = P(-1, 1)
        den = multiply(multiply(xm1, xm1), multiply(xm1, xm1))
        (rat, log) = check_hermite_identity(P(1), den)
        assert normalize(log[0]) == ()

    def test_shifted_linear_squared(self):
        # ∫ 1/(x + 2)^2 dx — different constant term.
        xp2 = P(2, 1)
        den = multiply(xp2, xp2)
        (rat, log) = check_hermite_identity(P(1), den)
        assert normalize(log[0]) == ()

    def test_numerator_is_derivative_shape(self):
        # ∫ 2/(x² - 1)²-like construction — verify algorithm handles
        # mixed numerator. Use a case crafted so the answer is rational.
        # Pick rat = 1/(x² + 1); then d/dx(rat) = -2x/(x² + 1)².
        # So integrate(-2x/(x²+1)², x) = 1/(x²+1) + C — purely rational.
        num = P(0, -2)  # -2x
        x2p1 = P(1, 0, 1)
        den = multiply(x2p1, x2p1)
        (rat, log) = check_hermite_identity(num, den)
        assert normalize(log[0]) == ()


# ---------------------------------------------------------------------------
# Mixed rational + log antiderivatives
# ---------------------------------------------------------------------------


class TestMixedRationalAndLog:
    """Integrands that decompose into a rational piece plus a log piece."""

    def test_mixed_over_linear_squared_times_other(self):
        # ∫ 1 / ((x - 1)² (x + 1)) dx has a rational part and a log part.
        # Rational part = -1/(2 (x - 1)); log part integrand has
        # denominator (x - 1)(x + 1) (squarefree).
        xm1 = P(-1, 1)
        xp1 = P(1, 1)
        den = multiply(multiply(xm1, xm1), xp1)
        (rat, log) = check_hermite_identity(P(1), den)
        # Log denominator must be squarefree.
        assert_squarefree(log[1])

    def test_higher_power_mixed(self):
        # ∫ x / (x - 1)^3 dx: has rational and log components.
        xm1 = P(-1, 1)
        den = multiply(xm1, multiply(xm1, xm1))
        (rat, log) = check_hermite_identity(P(0, 1), den)
        assert_squarefree(log[1])


# ---------------------------------------------------------------------------
# Already-squarefree inputs — nothing to peel
# ---------------------------------------------------------------------------


class TestAlreadySquarefree:
    def test_one_over_x_minus_one(self):
        # ∫ 1/(x - 1) dx — squarefree denom; no rational part.
        xm1 = P(-1, 1)
        (rat, log) = check_hermite_identity(P(1), xm1)
        assert normalize(rat[0]) == ()
        # Log integrand equals input.
        assert rat_equal(log, (P(1), xm1))

    def test_one_over_quadratic_irreducible(self):
        # ∫ 1/(x² + 1) dx — squarefree denom; the answer is arctan,
        # which Hermite alone doesn't touch. Rational part zero, log
        # integrand equals input.
        x2p1 = P(1, 0, 1)
        (rat, log) = check_hermite_identity(P(1), x2p1)
        assert normalize(rat[0]) == ()

    def test_linear_over_product_of_linears(self):
        # ∫ x / ((x - 1)(x + 1)) dx — squarefree denom.
        xm1 = P(-1, 1)
        xp1 = P(1, 1)
        den = multiply(xm1, xp1)
        (rat, log) = check_hermite_identity(P(0, 1), den)
        assert normalize(rat[0]) == ()


# ---------------------------------------------------------------------------
# Scalar / normalization edge cases
# ---------------------------------------------------------------------------


class TestNormalization:
    def test_non_monic_denominator_is_rescaled(self):
        # Denominator 2·(x - 1)² — leading coefficient folded into num.
        xm1 = P(-1, 1)
        den = multiply(P(2), multiply(xm1, xm1))
        (rat, log) = check_hermite_identity(P(1), den)
        assert normalize(log[0]) == ()

    def test_rational_coefficients_in_input(self):
        # Input already has Fraction coefficients in num — nothing
        # special, but exercise the path.
        num = (Fraction(1, 3),)
        xm1 = P(-1, 1)
        den = multiply(xm1, xm1)
        (rat, log) = check_hermite_identity(num, den)
        assert normalize(log[0]) == ()

    def test_zero_denominator_raises(self):
        with pytest.raises(ValueError):
            hermite_reduce(P(1), ())


# ---------------------------------------------------------------------------
# Repeated factors at multiple multiplicities
# ---------------------------------------------------------------------------


class TestMultipleRepeatedFactors:
    def test_product_of_two_squared_linears(self):
        # ∫ 1 / ((x - 1)² (x + 2)²) dx — both factors have multiplicity 2.
        xm1 = P(-1, 1)
        xp2 = P(2, 1)
        den = multiply(multiply(xm1, xm1), multiply(xp2, xp2))
        (rat, log) = check_hermite_identity(P(1), den)
        assert_squarefree(log[1])

    def test_mixed_multiplicities_2_and_3(self):
        # ∫ 1/((x - 1)²(x + 1)^3) dx — different multiplicities.
        xm1 = P(-1, 1)
        xp1 = P(1, 1)
        den = multiply(
            multiply(xm1, xm1), multiply(xp1, multiply(xp1, xp1))
        )
        (rat, log) = check_hermite_identity(P(1), den)
        assert_squarefree(log[1])
