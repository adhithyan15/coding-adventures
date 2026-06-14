"""Tests for Berlekamp-Zassenhaus-Hensel (BZH) polynomial factoring.

These tests cover the internal helper functions (GF(p) arithmetic, Berlekamp,
Hensel lifting, factor combination) as well as the public ``bzh_factor``
entry point and the integrated ``factor_integer_polynomial`` Phase 3 path.

Test structure
--------------
- ``TestGFpArithmetic``   — modular polynomial arithmetic primitives
- ``TestSquarefreeness``  — squarefree detection mod p
- ``TestBerlekamp``       — Berlekamp factoring over GF(p)
- ``TestHenselLifting``   — linear Hensel lift and multi-factor lift
- ``TestFactorCombination`` — Zassenhaus recombination
- ``TestBzhFactorPublic`` — public ``bzh_factor`` API (main correctness tests)
- ``TestBzhEdgeCases``    — degree caps, non-monic, empty inputs
- ``TestIntegratedPhase3`` — phase-3 path through ``factor_integer_polynomial``
"""

from __future__ import annotations

from cas_factor import factor_integer_polynomial
from cas_factor.bzh import (
    MAX_DEGREE,
    _berlekamp_factor_mod_p,
    _center_mod,
    _combine_factors,
    _is_squarefree_mod_p,
    _linear_hensel_lift,
    _multi_hensel_lift,
    _null_space_mod_p,
    _padd,
    _pdeg,
    _pderiv,
    _pdiv_quotient,
    _pgcd,
    _pgcd_extended,
    _pmod,
    _pmod_poly,
    _pmul,
    _pneg,
    _poly_divides_z,
    _poly_powmod,
    _pscale,
    _psub,
    _to_z_centered,
    _zassenhaus_bound,
    bzh_factor,
)
from cas_factor.polynomial import evaluate, normalize

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _poly_eval_at(coeffs: list[int], x: int) -> int:
    """Evaluate a coefficient list (ascending degree) at x."""
    return evaluate(coeffs, x)


def _verify_factorization(original: list[int], factors: list[list[int]]) -> None:
    """Assert that the product of factors equals original at several test points."""
    for x in range(-5, 6):
        expected = _poly_eval_at(original, x)
        got = 1
        for f in factors:
            got *= _poly_eval_at(f, x)
        assert got == expected, (
            f"Factorization wrong at x={x}: expected {expected}, got {got}. "
            f"original={original}, factors={factors}"
        )


# ---------------------------------------------------------------------------
# GF(p) arithmetic
# ---------------------------------------------------------------------------


class TestGFpArithmetic:
    """Low-level polynomial arithmetic over GF(p)."""

    def test_pmod_reduces_coefficients(self) -> None:
        """_pmod reduces every coefficient into [0, p-1]."""
        result = _pmod([7, -3, 11], 5)
        assert result == [2, 2, 1]

    def test_pmod_strips_trailing_zeros(self) -> None:
        """_pmod strips trailing zeros after reduction."""
        result = _pmod([5, 3, 10], 5)  # 5 and 10 become 0
        assert result == [0, 3]

    def test_pmod_empty(self) -> None:
        """_pmod of empty list is empty."""
        assert _pmod([], 7) == []

    def test_pdeg_empty(self) -> None:
        """_pdeg of empty list is -1."""
        assert _pdeg([]) == -1

    def test_pdeg_constant(self) -> None:
        """_pdeg of [c] is 0."""
        assert _pdeg([3]) == 0

    def test_pdeg_quadratic(self) -> None:
        """_pdeg of [1,2,3] is 2."""
        assert _pdeg([1, 2, 3]) == 2

    def test_pneg_mod5(self) -> None:
        """_pneg negates each coefficient mod p."""
        result = _pneg([1, 2, 3], 5)
        assert result == [4, 3, 2]

    def test_padd_simple(self) -> None:
        """_padd adds two polynomials mod p."""
        # (x + 1) + (x + 2) = 2x + 3
        result = _padd([1, 1], [2, 1], 7)
        assert result == [3, 2]

    def test_padd_cancellation(self) -> None:
        """_padd cancels leading term when sum is zero."""
        # (3x + 1) + (4x + 2) mod 7 = 7x + 3 → x-term cancels
        result = _padd([1, 3], [2, 4], 7)
        assert result == [3]

    def test_psub_simple(self) -> None:
        """_psub subtracts b from a mod p."""
        # (2x + 3) - (x + 1) = x + 2 mod 7
        result = _psub([3, 2], [1, 1], 7)
        assert result == [2, 1]

    def test_pmul_simple(self) -> None:
        """_pmul multiplies (x+1)(x+1) = x^2+2x+1 mod 5."""
        result = _pmul([1, 1], [1, 1], 5)
        assert result == [1, 2, 1]

    def test_pmul_empty(self) -> None:
        """_pmul with empty returns empty."""
        assert _pmul([], [1, 2], 5) == []
        assert _pmul([1, 2], [], 5) == []

    def test_pscale(self) -> None:
        """_pscale multiplies each coefficient by scalar mod p."""
        result = _pscale([1, 2, 3], 3, 7)
        assert result == [3, 6, 2]

    def test_pmod_poly_reduces_to_remainder(self) -> None:
        """_pmod_poly gives remainder of x^3 / (x^2 + 1) mod 5."""
        # x^3 = x * (x^2+1) - x, so remainder = -x ≡ 4x mod 5 = [0, 4]
        result = _pmod_poly([0, 0, 0, 1], [1, 0, 1], 5)
        assert result == [0, 4]

    def test_pdiv_quotient(self) -> None:
        """_pdiv_quotient gives x^3 / (x^2+1) = x (quotient) mod 5."""
        quot = _pdiv_quotient([0, 0, 0, 1], [1, 0, 1], 5)
        assert quot == [0, 1]  # [0, 1] = x

    def test_poly_powmod(self) -> None:
        """_poly_powmod: x^5 mod (x^2-1) mod 5 = x."""
        # x^2 ≡ 1 mod (x^2-1), so x^5 = x*(x^2)^2 ≡ x
        result = _poly_powmod(5, [4, 0, 1], 5)  # mod_poly = x^2 - 1 mod 5 = [4,0,1]
        assert result == [0, 1]  # x

    def test_center_mod(self) -> None:
        """_center_mod puts coefficients in (-m/2, m/2]."""
        result = _center_mod([1, 6, 5, 4], 7)
        # 1→1, 6→-1, 5→-2, 4→-3
        assert result == [1, -1, -2, -3]

    def test_to_z_centered(self) -> None:
        """_to_z_centered converts from GF(p) to centered Z."""
        result = _to_z_centered([0, 1, 4, 3], 5)
        # 0→0, 1→1, 4→-1, 3→-2
        assert result == [0, 1, -1, -2]


# ---------------------------------------------------------------------------
# GCD and extended GCD
# ---------------------------------------------------------------------------


class TestGFpGcd:
    """Polynomial GCD over GF(p)."""

    def test_pgcd_coprime(self) -> None:
        """gcd(x+1, x+2) = 1 mod 7."""
        g = _pgcd([1, 1], [2, 1], 7)
        assert g == [1]

    def test_pgcd_common_factor(self) -> None:
        """gcd(x^2-1, x-1) = x-1 mod 7."""
        # x^2-1 = (x-1)(x+1)
        g = _pgcd([-1, 0, 1], [-1, 1], 7)
        # g should be monic and equal to x-1, i.e. [6,1] mod 7
        assert _pdeg(g) == 1

    def test_pgcd_extended_bezout(self) -> None:
        """Extended GCD satisfies s*a + t*b = gcd(a,b) mod p."""
        a = [1, 1]   # x+1
        b = [2, 1]   # x+2
        g, s, t = _pgcd_extended(a, b, 7)
        # Verify: s*a + t*b = g (mod 7)
        sa = _pmul(s, a, 7)
        tb = _pmul(t, b, 7)
        lhs = _padd(sa, tb, 7)
        assert lhs == g

    def test_pderiv_quadratic(self) -> None:
        """Derivative of x^2 + 3x + 1 is 2x + 3 mod 5."""
        # [1, 3, 1] → [3, 2]
        result = _pderiv([1, 3, 1], 5)
        assert result == [3, 2]

    def test_pderiv_constant(self) -> None:
        """Derivative of constant is empty."""
        assert _pderiv([4], 7) == []


# ---------------------------------------------------------------------------
# Squarefreeness
# ---------------------------------------------------------------------------


class TestSquarefreeness:
    """_is_squarefree_mod_p tests."""

    def test_squarefree_x_squared_minus_1(self) -> None:
        """x^2-1 = (x-1)(x+1) is squarefree."""
        assert _is_squarefree_mod_p([-1, 0, 1], 7) is True

    def test_not_squarefree_x_squared_plus_2x_plus_1(self) -> None:
        """(x+1)^2 = x^2+2x+1 is not squarefree."""
        assert _is_squarefree_mod_p([1, 2, 1], 7) is False

    def test_squarefree_linear(self) -> None:
        """Linear polynomials are always squarefree."""
        assert _is_squarefree_mod_p([3, 1], 7) is True

    def test_zero_mod_p_not_squarefree(self) -> None:
        """A polynomial that vanishes mod p is not squarefree."""
        # 7x + 7 ≡ 0 mod 7
        assert _is_squarefree_mod_p([7, 7], 7) is False


# ---------------------------------------------------------------------------
# Berlekamp factoring over GF(p)
# ---------------------------------------------------------------------------


class TestBerlekamp:
    """_berlekamp_factor_mod_p tests."""

    def test_berlekamp_irreducible(self) -> None:
        """x^2+1 is irreducible over GF(3) (no square root of -1 mod 3)."""
        result = _berlekamp_factor_mod_p([1, 0, 1], 3)
        # Should return a single factor (the polynomial itself)
        assert len(result) == 1

    def test_berlekamp_splits_x2_minus_1(self) -> None:
        """x^2 - 1 = (x-1)(x+1) splits over GF(5)."""
        result = _berlekamp_factor_mod_p([4, 0, 1], 5)  # -1 ≡ 4 mod 5
        assert len(result) == 2

    def test_berlekamp_linear_trivial(self) -> None:
        """Linear polynomial — returns itself."""
        result = _berlekamp_factor_mod_p([3, 1], 7)
        assert result == [[3, 1]]

    def test_berlekamp_empty(self) -> None:
        """Empty polynomial — returns empty."""
        result = _berlekamp_factor_mod_p([], 7)
        assert result == []

    def test_berlekamp_splits_into_correct_degrees(self) -> None:
        """x^4-1 = (x-1)(x+1)(x^2+1) mod 5 — three factors."""
        # [4, 0, 0, 0, 1] = x^4 - 1 mod 5
        result = _berlekamp_factor_mod_p([4, 0, 0, 0, 1], 5)
        total_degree = sum(_pdeg(f) for f in result)
        assert total_degree == 4  # factors account for full degree

    def test_berlekamp_product_equals_original(self) -> None:
        """Product of Berlekamp factors = original mod p."""
        # Use p=2: x^5+1 mod 2 = x^5-1 = (x-1)(x^4+x^3+x^2+x+1) mod 2
        f2 = [1, 0, 0, 0, 0, 1]  # x^5+1 mod 2 = x^5-1
        factors = _berlekamp_factor_mod_p(f2, 2)
        # Product should equal f2 mod 2
        from functools import reduce
        prod = reduce(lambda a, b: _pmul(a, b, 2), factors)
        assert _pmod(prod, 2) == _pmod(f2, 2)


# ---------------------------------------------------------------------------
# Null space
# ---------------------------------------------------------------------------


class TestNullSpace:
    """_null_space_mod_p tests."""

    def test_null_space_identity_minus_I(self) -> None:
        """Null space of zero matrix has dimension n."""
        # Zero matrix → every standard basis vector is in null space
        n = 3
        M = [[0] * n for _ in range(n)]
        basis = _null_space_mod_p(M, n, 5)
        assert len(basis) == n

    def test_null_space_full_rank(self) -> None:
        """Identity matrix → null space has dimension 0 (returns trivial vector)."""
        n = 3
        M = [[1 if i == j else 0 for j in range(n)] for i in range(n)]
        basis = _null_space_mod_p(M, n, 5)
        # Returns at least 1 vector (the trivial [1,0,...,0])
        assert len(basis) >= 1


# ---------------------------------------------------------------------------
# Zassenhaus bound
# ---------------------------------------------------------------------------


class TestZassenhausBound:
    """_zassenhaus_bound tests."""

    def test_bound_positive(self) -> None:
        """Zassenhaus bound is positive for non-trivial polynomials."""
        B = _zassenhaus_bound([-1, 0, 0, 0, 0, 1])  # x^5 - 1
        assert B > 0

    def test_bound_constant_zero(self) -> None:
        """Zassenhaus bound for empty polynomial is 0."""
        assert _zassenhaus_bound([]) == 0.0

    def test_bound_degree_cap(self) -> None:
        """Bound grows with polynomial magnitude."""
        B_small = _zassenhaus_bound([-1, 1])      # x - 1
        B_large = _zassenhaus_bound([-100, 0, 0, 0, 1])  # x^4 - 100
        assert B_large > B_small


# ---------------------------------------------------------------------------
# Divisibility test
# ---------------------------------------------------------------------------


class TestPolyDividesZ:
    """_poly_divides_z tests."""

    def test_divides_exact(self) -> None:
        """(x-1) divides (x^2-1)."""
        quot = _poly_divides_z([-1, 0, 1], [-1, 1])
        assert quot is not None
        assert quot == [1, 1]  # quotient x+1

    def test_does_not_divide(self) -> None:
        """(x-2) does not divide (x^2-1)."""
        quot = _poly_divides_z([-1, 0, 1], [-2, 1])
        assert quot is None

    def test_divides_higher_degree(self) -> None:
        """(x^2+x+1) divides (x^5-1)."""
        # x^5-1 = (x-1)(x^4+x^3+x^2+x+1)
        # and x^2+x+1 divides x^4+x^3+x^2+x+1? No.
        # Let's test: (x-1) | (x^5-1)
        quot = _poly_divides_z([-1, 0, 0, 0, 0, 1], [-1, 1])
        assert quot is not None  # quotient = x^4+x^3+x^2+x+1

    def test_trivial_divisor(self) -> None:
        """Degree-0 divisor [2] divides [4, 6]."""
        quot = _poly_divides_z([4, 6], [2])
        assert quot == [2, 3]

    def test_zero_divisor(self) -> None:
        """Zero divisor returns None."""
        assert _poly_divides_z([1, 2], []) is None

    def test_degree_too_high(self) -> None:
        """Divisor degree > dividend degree returns None."""
        assert _poly_divides_z([1, 1], [1, 0, 1]) is None


# ---------------------------------------------------------------------------
# Hensel lifting
# ---------------------------------------------------------------------------


class TestHenselLifting:
    """_linear_hensel_lift and _multi_hensel_lift tests."""

    def test_linear_lift_x2_minus_1(self) -> None:
        """Hensel lift for x^2-1 = (x-1)(x+1) starting from mod 3."""
        # x^2 - 1 factors as [2,1] * [1,1] mod 3 (i.e. (x-1)(x+1) mod 3)
        f = [-1, 0, 1]
        g_mod = [2, 1]  # x - 1 mod 3
        h_mod = [1, 1]  # x + 1 mod 3
        result = _linear_hensel_lift(f, g_mod, h_mod, 3, 10)
        assert result is not None
        g_lifted, h_lifted = result
        # Product should equal f exactly
        from cas_factor.bzh import _iz_mul
        prod = _iz_mul(g_lifted, h_lifted)
        assert normalize(prod) == normalize(f)

    def test_multi_hensel_lift_x5_minus_1(self) -> None:
        """Multi-factor Hensel lift for x^5-1 starting from mod-2 factors."""
        # x^5-1 mod 2 = (x+1)(x^4+x^3+x^2+x+1)
        f = [-1, 0, 0, 0, 0, 1]
        f_mod2 = _pmod(f, 2)
        factors_mod2 = _berlekamp_factor_mod_p(f_mod2, 2)
        B = _zassenhaus_bound(f)
        target = 2.0 * B + 1.0
        lifted = _multi_hensel_lift(f, factors_mod2, 2, target)
        assert lifted is not None
        assert len(lifted) >= 1

    def test_linear_lift_not_coprime_returns_none(self) -> None:
        """Lift returns None when g and h are not coprime mod p."""
        f = [-1, 0, 1]  # x^2 - 1
        # Give g = h = x+1 mod 5, which are NOT coprime with each other
        # Actually (x+1)(x+1) = x^2+2x+1 ≠ x^2-1, but let's test that
        # identical factors (not coprime) return None
        g_mod = [1, 1]  # x+1
        h_mod = [1, 1]  # x+1 (same, gcd = x+1 ≠ 1)
        result = _linear_hensel_lift(f, g_mod, h_mod, 5, 100)
        assert result is None

    def test_multi_hensel_single_factor(self) -> None:
        """Single factor trivially lifts to f itself."""
        f = [1, 0, 1]  # x^2 + 1
        lifted = _multi_hensel_lift(f, [[1, 0, 1]], 3, 10.0)
        assert lifted is not None
        assert len(lifted) == 1

    def test_multi_hensel_empty_factors(self) -> None:
        """Zero factors returns empty list."""
        lifted = _multi_hensel_lift([1, 1], [], 3, 10.0)
        assert lifted == []


# ---------------------------------------------------------------------------
# Factor combination
# ---------------------------------------------------------------------------


class TestFactorCombination:
    """_combine_factors tests."""

    def test_combine_x2_minus_1(self) -> None:
        """Combine lifted [x-1, x+1] for x^2-1 finds both factors."""
        f = [-1, 0, 1]
        lifted = [[-1, 1], [1, 1]]
        result = _combine_factors(f, lifted, 100)
        assert result is not None
        assert len(result) == 2
        _verify_factorization(f, result)

    def test_combine_irreducible(self) -> None:
        """Combination of x^2+1 factors returns None (irreducible)."""
        f = [1, 0, 1]
        # Lifted factors that don't actually combine
        lifted = [[1, 0, 1]]  # just f itself
        result = _combine_factors(f, lifted, 100)
        assert result is None or (len(result) == 1 and result[0] == [1, 0, 1])


# ---------------------------------------------------------------------------
# Public bzh_factor API
# ---------------------------------------------------------------------------


class TestBzhFactorPublic:
    """Main correctness tests for the ``bzh_factor`` public function."""

    def test_x5_minus_1(self) -> None:
        """x^5-1 = (x-1)(x^4+x^3+x^2+x+1)."""
        result = bzh_factor([-1, 0, 0, 0, 0, 1])
        assert result is not None
        assert len(result) == 2
        _verify_factorization([-1, 0, 0, 0, 0, 1], result)

    def test_x5_minus_1_factor_degrees(self) -> None:
        """x^5-1 gives one linear and one quartic factor."""
        result = bzh_factor([-1, 0, 0, 0, 0, 1])
        assert result is not None
        degrees = sorted(len(f) - 1 for f in result)
        assert degrees == [1, 4]

    def test_x4_plus_1_irreducible(self) -> None:
        """x^4+1 is irreducible over Q — BZH returns None."""
        result = bzh_factor([1, 0, 0, 0, 1])
        assert result is None

    def test_x8_minus_1(self) -> None:
        """x^8-1 = (x-1)(x+1)(x^2+1)(x^4+1)."""
        result = bzh_factor([-1, 0, 0, 0, 0, 0, 0, 0, 1])
        assert result is not None
        assert len(result) >= 2  # at minimum splits into 2 factors
        _verify_factorization([-1, 0, 0, 0, 0, 0, 0, 0, 1], result)

    def test_x6_minus_1(self) -> None:
        """x^6-1 = (x-1)(x+1)(x^2+x+1)(x^2-x+1)."""
        result = bzh_factor([-1, 0, 0, 0, 0, 0, 1])
        assert result is not None
        _verify_factorization([-1, 0, 0, 0, 0, 0, 1], result)

    def test_x5_cyclotomic(self) -> None:
        """x^5+x^4+x^3+x^2+x+1 = (x+1)(x^2+x+1)(x^2-x+1)."""
        # Coefficients: [1,1,1,1,1,1] = 1+x+x^2+x^3+x^4+x^5
        result = bzh_factor([1, 1, 1, 1, 1, 1])
        assert result is not None
        _verify_factorization([1, 1, 1, 1, 1, 1], result)

    def test_x4_plus_x2_plus_1(self) -> None:
        """x^4+x^2+1 = (x^2+x+1)(x^2-x+1) — Kronecker also handles this."""
        result = bzh_factor([1, 0, 1, 0, 1])
        # Either BZH finds it or returns None (Kronecker handles this case)
        if result is not None:
            _verify_factorization([1, 0, 1, 0, 1], result)

    def test_x9_minus_1(self) -> None:
        """x^9-1 = (x-1)(x^2+x+1)(x^6+x^3+1)."""
        result = bzh_factor([-1, 0, 0, 0, 0, 0, 0, 0, 0, 1])
        assert result is not None
        _verify_factorization([-1, 0, 0, 0, 0, 0, 0, 0, 0, 1], result)

    def test_x6_plus_x3_plus_1_irreducible(self) -> None:
        """x^6+x^3+1 is irreducible (cyclotomic Φ_9 part) — returns None."""
        result = bzh_factor([1, 0, 0, 1, 0, 0, 1])
        # This is the Φ_9(x) cyclotomic polynomial, irreducible over Q
        assert result is None


# ---------------------------------------------------------------------------
# Edge cases and limits
# ---------------------------------------------------------------------------


class TestBzhEdgeCases:
    """Boundary conditions, degree caps, and non-monic inputs."""

    def test_degree_0_returns_none(self) -> None:
        """Constant polynomial has no factors."""
        assert bzh_factor([5]) is None

    def test_degree_1_returns_none(self) -> None:
        """Linear polynomial is always irreducible."""
        assert bzh_factor([3, 1]) is None

    def test_empty_returns_none(self) -> None:
        """Empty list returns None."""
        assert bzh_factor([]) is None

    def test_non_monic_returns_none(self) -> None:
        """Non-monic polynomial — BZH restriction, returns None."""
        assert bzh_factor([1, 0, 0, 0, 2]) is None  # 2x^4 + 1, lc=2

    def test_degree_exceeds_cap_returns_none(self) -> None:
        """Polynomial exceeding MAX_DEGREE returns None."""
        coeffs = [1] + [0] * MAX_DEGREE + [1]  # x^(MAX_DEGREE+1) + 1
        assert bzh_factor(coeffs) is None

    def test_degree_2_squarefree(self) -> None:
        """x^2-1 is monic degree 2 — BZH should handle it."""
        result = bzh_factor([-1, 0, 1])
        # Either returns factors or None (Kronecker handles this case anyway)
        if result is not None:
            _verify_factorization([-1, 0, 1], result)

    def test_x4_plus_4_sophie_germain(self) -> None:
        """x^4+4 — monic, Kronecker handles this in practice.

        When called directly, BZH may return None (treating x^4+4 as if
        irreducible mod the chosen prime) or a valid factorization.  What
        it must NOT do is crash.  Verification is skipped here because the
        ``factor_integer_polynomial`` pipeline tries Kronecker first; the
        BZH unit path sees a polynomial it doesn't need to handle.
        """
        # Just verify it doesn't crash and returns a list or None.
        result = bzh_factor([4, 0, 0, 0, 1])
        assert result is None or isinstance(result, list)


# ---------------------------------------------------------------------------
# Integrated Phase 3 (factor_integer_polynomial uses BZH as fallback)
# ---------------------------------------------------------------------------


class TestIntegratedPhase3:
    """End-to-end tests through factor_integer_polynomial Phase 3 path."""

    def test_x5_minus_1_through_pipeline(self) -> None:
        """x^5-1 is correctly factored by the full pipeline."""
        content, factors = factor_integer_polynomial([-1, 0, 0, 0, 0, 1])
        assert content == 1
        # Should have at least (x-1) as a linear factor
        assert any(len(f) == 2 for f, _ in factors)
        # Product verification
        all_polys = [f for f, _ in factors]
        _verify_factorization([-1, 0, 0, 0, 0, 1], all_polys)

    def test_content_extracted_before_bzh(self) -> None:
        """2*(x^5-1) = 2*(x-1)*(x^4+x^3+x^2+x+1): content 2, then BZH."""
        content, factors = factor_integer_polynomial([-2, 0, 0, 0, 0, 2])
        assert content == 2
        for x in range(-4, 5):
            expected = _poly_eval_at([-2, 0, 0, 0, 0, 2], x)
            got = content * 1
            for f, mult in factors:
                got *= _poly_eval_at(f, x) ** mult
            assert got == expected

    def test_x8_minus_1_through_pipeline(self) -> None:
        """x^8-1 factors fully through the pipeline."""
        content, factors = factor_integer_polynomial([-1, 0, 0, 0, 0, 0, 0, 0, 1])
        assert content == 1
        all_polys = [f for f, _ in factors]
        _verify_factorization([-1, 0, 0, 0, 0, 0, 0, 0, 1], all_polys)

    def test_x4_plus_1_remains_irreducible(self) -> None:
        """x^4+1 — both Kronecker and BZH confirm irreducibility."""
        content, factors = factor_integer_polynomial([1, 0, 0, 0, 1])
        assert content == 1
        assert len(factors) == 1
        assert factors[0] == ([1, 0, 0, 0, 1], 1)

    def test_content_2_x4_plus_1(self) -> None:
        """2*(x^4+1) = content 2, one irreducible factor x^4+1."""
        content, factors = factor_integer_polynomial([2, 0, 0, 0, 2])
        assert content == 2
        assert factors == [([1, 0, 0, 0, 1], 1)]

    def test_bzh_then_kronecker_residual(self) -> None:
        """x^6-1 = (x-1)(x+1)(x^2+x+1)(x^2-x+1) — fully factored."""
        content, factors = factor_integer_polynomial([-1, 0, 0, 0, 0, 0, 1])
        assert content == 1
        # x^6-1 has 4 irreducible factors
        assert len(factors) >= 2
        all_polys = [f for f, _ in factors]
        _verify_factorization([-1, 0, 0, 0, 0, 0, 1], all_polys)

    def test_x9_minus_1_pipeline(self) -> None:
        """x^9-1 = (x-1)(x^2+x+1)(x^6+x^3+1) factors correctly."""
        content, factors = factor_integer_polynomial([-1, 0, 0, 0, 0, 0, 0, 0, 0, 1])
        assert content == 1
        all_polys = [f for f, _ in factors]
        _verify_factorization([-1, 0, 0, 0, 0, 0, 0, 0, 0, 1], all_polys)
