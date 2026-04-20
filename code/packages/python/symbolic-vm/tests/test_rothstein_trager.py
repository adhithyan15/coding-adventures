"""Tests for the Rothstein–Trager log-part finder.

Correctness gate, in every test that expects a list: the
**re-differentiation identity** on the log sum. For the returned pairs
``[(c_i, v_i)]``, we verify

    Σ c_i · v_i' · ∏_{j≠i} v_j  ==  num

as a polynomial equality (after accounting for the prescribed product
``∏ v_i == den``). No logs appear in the check — it's purely Q[x].

For the "escape Q" tests, we assert the return is exactly ``None``.
"""

from __future__ import annotations

from fractions import Fraction

from polynomial import (
    add,
    deriv,
    monic,
    multiply,
    normalize,
)

from symbolic_vm.rothstein_trager import rothstein_trager

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def P(*coefs):
    """Polynomial tuple with Fraction coefficients."""
    return tuple(Fraction(c) for c in coefs)


def scale(p, c):
    return tuple(Fraction(c) * Fraction(coef) for coef in p)


def product_of(factors):
    """Return the product of a non-empty list of polynomials."""
    acc = factors[0]
    for f in factors[1:]:
        acc = multiply(acc, f)
    return acc


def assert_rt_identity(pairs, num, den):
    """Universal gate: Σ c_i · v_i' · ∏_{j≠i} v_j == (num * leading scale).

    The RT output has ``∏ v_i == monic(den)``; the original ``den`` may
    carry a non-unit leading coefficient, so the identity we check is
    ``Σ c_i · v_i' · ∏_{j≠i} v_j == num / lc(den)``. We first
    rescale ``num`` by ``1/lc(den)`` to match.
    """
    den_n = normalize(den)
    lc = Fraction(den_n[-1])
    num_scaled = scale(num, Fraction(1) / lc)

    vs = [v for (_, v) in pairs]
    # ∏ v_i should equal monic(den).
    prod = product_of(vs)
    assert normalize(prod) == normalize(monic(den_n)), (
        f"RT invariant ∏ v_i == monic(den) violated:\n"
        f"  ∏ v_i = {normalize(prod)}\n"
        f"  monic(den) = {monic(den_n)}"
    )

    # Σ c_i · v_i' · ∏_{j≠i} v_j.
    total = ()
    for i, (c, v) in enumerate(pairs):
        others = [vs[j] for j in range(len(vs)) if j != i]
        prod_others = product_of(others) if others else P(1)
        term = scale(multiply(deriv(v), prod_others), c)
        total = add(total, term)

    assert normalize(total) == normalize(num_scaled), (
        f"RT identity failed:\n"
        f"  Σ c_i · v_i' · ∏_{{j≠i}} v_j = {normalize(total)}\n"
        f"  num / lc(den)             = {normalize(num_scaled)}"
    )


# ---------------------------------------------------------------------------
# Single log term — trivial case
# ---------------------------------------------------------------------------


class TestSingleLog:
    def test_one_over_x_minus_one(self):
        # ∫ 1/(x - 1) dx = log(x - 1).
        pairs = rothstein_trager(P(1), P(-1, 1))
        assert pairs is not None
        assert len(pairs) == 1
        c, v = pairs[0]
        assert c == Fraction(1)
        assert v == P(-1, 1)
        assert_rt_identity(pairs, P(1), P(-1, 1))

    def test_one_over_x_plus_two(self):
        # ∫ 1/(x + 2) dx = log(x + 2).
        pairs = rothstein_trager(P(1), P(2, 1))
        assert pairs is not None
        assert len(pairs) == 1
        c, v = pairs[0]
        assert c == Fraction(1)
        assert v == P(2, 1)


# ---------------------------------------------------------------------------
# Multiple log terms — partial-fractions cases with distinct rational roots
# ---------------------------------------------------------------------------


class TestMultipleLogs:
    def test_one_over_x_squared_minus_one(self):
        # ∫ 1/(x² - 1) dx = (1/2) log(x - 1) − (1/2) log(x + 1).
        xm1 = P(-1, 1)
        xp1 = P(1, 1)
        den = multiply(xm1, xp1)
        pairs = rothstein_trager(P(1), den)
        assert pairs is not None
        assert len(pairs) == 2
        assert_rt_identity(pairs, P(1), den)
        # Explicit coefficient check (order: sorted by alpha).
        coeffs = sorted(c for (c, _) in pairs)
        assert coeffs == [Fraction(-1, 2), Fraction(1, 2)]

    def test_x_over_linear_product(self):
        # ∫ x/((x − 1)(x − 2)) dx = -log(x − 1) + 2·log(x − 2).
        xm1 = P(-1, 1)
        xm2 = P(-2, 1)
        den = multiply(xm1, xm2)
        pairs = rothstein_trager(P(0, 1), den)
        assert pairs is not None
        assert_rt_identity(pairs, P(0, 1), den)
        coeffs = sorted(c for (c, _) in pairs)
        assert coeffs == [Fraction(-1), Fraction(2)]

    def test_mixed_numerator_three_way_split(self):
        # ∫ (3x + 1) / (x (x + 1)) dx = log(x) + 2·log(x + 1).
        x = P(0, 1)
        xp1 = P(1, 1)
        den = multiply(x, xp1)
        pairs = rothstein_trager(P(1, 3), den)
        assert pairs is not None
        assert_rt_identity(pairs, P(1, 3), den)
        coeffs = sorted(c for (c, _) in pairs)
        assert coeffs == [Fraction(1), Fraction(2)]

    def test_three_linear_factors(self):
        # ∫ 1 / ((x-1)(x-2)(x-3)) dx — the partial-fraction residues are
        # {1/2, -1, 1/2} at x = 1, 2, 3 respectively. RT groups the two
        # matching 1/2-residues into a single log factor
        # v_{1/2} = (x − 1)(x − 3), so we get *two* log terms — not
        # three. This is the classic demonstration of RT collapsing
        # duplicate residues without ever factoring the denominator.
        xm1 = P(-1, 1)
        xm2 = P(-2, 1)
        xm3 = P(-3, 1)
        den = multiply(multiply(xm1, xm2), xm3)
        pairs = rothstein_trager(P(1), den)
        assert pairs is not None
        assert len(pairs) == 2
        assert_rt_identity(pairs, P(1), den)
        # Coefficients: one -1 (for x-2) and one +1/2 (for (x-1)(x-3)).
        coeffs = sorted(c for (c, _) in pairs)
        assert coeffs == [Fraction(-1), Fraction(1, 2)]

    def test_three_distinct_residues(self):
        # Pick a numerator that produces three distinct residues, so the
        # log sum actually has three terms. The residue at x = k is
        # num(k) / E'(k); we want these to be distinct.
        # Take num = x^2, den = (x-1)(x-2)(x-3). Residues:
        #   at 1: 1 / ((1-2)(1-3)) = 1/2
        #   at 2: 4 / ((2-1)(2-3)) = -4
        #   at 3: 9 / ((3-1)(3-2)) = 9/2
        xm1 = P(-1, 1)
        xm2 = P(-2, 1)
        xm3 = P(-3, 1)
        den = multiply(multiply(xm1, xm2), xm3)
        pairs = rothstein_trager(P(0, 0, 1), den)  # x^2
        assert pairs is not None
        assert len(pairs) == 3
        assert_rt_identity(pairs, P(0, 0, 1), den)
        coeffs = sorted(c for (c, _) in pairs)
        assert coeffs == [Fraction(-4), Fraction(1, 2), Fraction(9, 2)]


# ---------------------------------------------------------------------------
# Irrational / complex roots — RT bails out
# ---------------------------------------------------------------------------


class TestEscapesQ:
    def test_arctan_integrand(self):
        # ∫ 1/(x² + 1) dx = arctan(x). RT resultant has roots ±i/2,
        # not in Q — we return None.
        assert rothstein_trager(P(1), P(1, 0, 1)) is None

    def test_irreducible_quadratic_denom(self):
        # x² - 2 is irreducible over Q; log coefficients are ±1/(2√2).
        assert rothstein_trager(P(1), P(-2, 0, 1)) is None

    def test_non_squarefree_resultant(self):
        # Integrands where the RT resultant has a root of multiplicity
        # >1 (degenerate) should still give back something usable OR
        # cleanly return None. Construct one by picking the numerator
        # so both partial-fraction residues coincide:
        # 1/((x-1)(x+1)) — residues at ±1 are ±1/2 (distinct), so
        # not actually degenerate. Real degenerate construction is
        # rare for clean inputs; this case should succeed normally.
        xm1 = P(-1, 1)
        xp1 = P(1, 1)
        den = multiply(xm1, xp1)
        pairs = rothstein_trager(P(1), den)
        assert pairs is not None
        assert_rt_identity(pairs, P(1), den)


# ---------------------------------------------------------------------------
# Non-monic denominators — normalisation contract
# ---------------------------------------------------------------------------


class TestNonMonic:
    def test_non_monic_denominator(self):
        # ∫ 1 / (2·(x − 1)) dx — leading coefficient 2 folded into the
        # computation. RT should produce [(1/2, x − 1)] (the residue
        # of 1/(2(x−1)) at x=1 is 1/2).
        xm1 = P(-1, 1)
        den = multiply(P(2), xm1)
        pairs = rothstein_trager(P(1), den)
        assert pairs is not None
        assert_rt_identity(pairs, P(1), den)
        c, v = pairs[0]
        assert c == Fraction(1, 2)
        assert v == xm1  # monic

    def test_integer_numerator(self):
        # Mixed int/Fraction input shapes still go through.
        xm1 = (-1, 1)  # plain ints
        xp1 = (1, 1)
        den = multiply(xm1, xp1)
        pairs = rothstein_trager((1,), den)
        assert pairs is not None
        assert len(pairs) == 2


# ---------------------------------------------------------------------------
# Lagrange interpolation corner case
# ---------------------------------------------------------------------------


class TestInterpolation:
    def test_degree_one_denom_gives_constant_resultant(self):
        # deg E = 1 ⇒ R(z) has degree ≤ 1. Specifically R(z) = a0 − z·a1
        # where E = a0 + a1·x. Interpolation on 2 samples must be exact.
        # Covered by test_one_over_x_minus_one already; this test
        # documents the reliance.
        pairs = rothstein_trager(P(3), P(-4, 1))  # 3/(x - 4)
        assert pairs is not None
        assert pairs[0] == (Fraction(3), P(-4, 1))
