"""
Comprehensive test suite for the polynomial_native extension.

Tests cover all 12 exported functions:
  - normalize, degree
  - zero, one
  - add, subtract, multiply
  - divmod_poly, divide, modulo
  - evaluate
  - gcd

Polynomials are represented as list[float] where index == degree:
  [a0, a1, a2, ...]  means  a0 + a1*x + a2*x^2 + ...

At least 25 tests are included to ensure thorough coverage.
"""

import math
import pytest

import polynomial_native as pn


# =========================================================================
# Normalize tests
# =========================================================================


class TestNormalize:
    """Tests for normalize(poly) — strips trailing near-zero coefficients."""

    def test_normalize_trailing_zeros(self) -> None:
        # [1.0, 0.0, 0.0] represents the constant 1; high-degree zeros stripped.
        assert pn.normalize([1.0, 0.0, 0.0]) == [1.0]

    def test_normalize_already_normalized(self) -> None:
        # No trailing zeros — result unchanged.
        assert pn.normalize([1.0, 2.0, 3.0]) == [1.0, 2.0, 3.0]

    def test_normalize_zero_polynomial(self) -> None:
        # [0.0] is the zero polynomial; all zeros become [].
        assert pn.normalize([0.0]) == []

    def test_normalize_empty(self) -> None:
        # Empty is already normalized.
        assert pn.normalize([]) == []

    def test_normalize_multiple_trailing_zeros(self) -> None:
        # Keep the first non-zero coefficient.
        assert pn.normalize([5.0, 0.0, 0.0, 0.0]) == [5.0]

    def test_normalize_middle_zeros_kept(self) -> None:
        # Zeros in the middle are NOT stripped — only trailing ones are.
        result = pn.normalize([1.0, 0.0, 1.0])
        assert result == [1.0, 0.0, 1.0]


# =========================================================================
# Degree tests
# =========================================================================


class TestDegree:
    """Tests for degree(poly) — returns the degree as an integer."""

    def test_degree_constant(self) -> None:
        # [7.0] is degree 0 — the constant 7.
        assert pn.degree([7.0]) == 0

    def test_degree_linear(self) -> None:
        # [0.0, 1.0] is degree 1 — the polynomial x.
        assert pn.degree([0.0, 1.0]) == 1

    def test_degree_quadratic(self) -> None:
        # [3.0, 0.0, 2.0] is degree 2 — 3 + 2x^2.
        assert pn.degree([3.0, 0.0, 2.0]) == 2

    def test_degree_zero_poly(self) -> None:
        # The zero polynomial has degree 0 by convention (not -1 or -inf).
        assert pn.degree([0.0]) == 0
        assert pn.degree([]) == 0

    def test_degree_strips_trailing_zeros(self) -> None:
        # [1.0, 0.0, 0.0] is actually degree 0 after normalization.
        assert pn.degree([1.0, 0.0, 0.0]) == 0


# =========================================================================
# Zero and One tests
# =========================================================================


class TestZeroOne:
    """Tests for zero() and one() — the additive and multiplicative identities."""

    def test_zero(self) -> None:
        # The zero polynomial is the additive identity.
        z = pn.zero()
        assert z == [0.0]

    def test_one(self) -> None:
        # The one polynomial is the multiplicative identity.
        o = pn.one()
        assert o == [1.0]

    def test_zero_is_additive_identity(self) -> None:
        # add(zero, p) == p for any polynomial p.
        p = [3.0, 2.0, 1.0]
        result = pn.add(pn.zero(), p)
        assert result == p

    def test_one_is_multiplicative_identity(self) -> None:
        # multiply(one, p) == p for any polynomial p.
        p = [3.0, 2.0, 1.0]
        result = pn.multiply(pn.one(), p)
        assert result == p


# =========================================================================
# Add tests
# =========================================================================


class TestAdd:
    """Tests for add(a, b) — coefficient-by-coefficient addition."""

    def test_add_basic(self) -> None:
        # [1, 2, 3] + [4, 5] = [5, 7, 3]
        #  (1+4, 2+5, 3+0)
        result = pn.add([1.0, 2.0, 3.0], [4.0, 5.0])
        assert result == [5.0, 7.0, 3.0]

    def test_add_same_length(self) -> None:
        # Equal length: simple element-wise addition.
        result = pn.add([1.0, 1.0], [2.0, 3.0])
        assert result == [3.0, 4.0]

    def test_add_with_zero(self) -> None:
        # Adding zero polynomial returns the original.
        p = [1.0, 2.0, 3.0]
        assert pn.add(p, [0.0]) == p

    def test_add_cancellation(self) -> None:
        # a + (-a) = 0
        result = pn.add([1.0, 2.0], [-1.0, -2.0])
        assert result == []

    def test_add_commutativity(self) -> None:
        # add(a, b) == add(b, a)
        a = [1.0, 2.0, 3.0]
        b = [4.0, 5.0]
        assert pn.add(a, b) == pn.add(b, a)


# =========================================================================
# Subtract tests
# =========================================================================


class TestSubtract:
    """Tests for subtract(a, b) — coefficient-by-coefficient subtraction."""

    def test_subtract_basic(self) -> None:
        # [5, 7, 3] - [1, 2, 3] = [4, 5, 0] → normalized → [4, 5]
        result = pn.subtract([5.0, 7.0, 3.0], [1.0, 2.0, 3.0])
        assert result == [4.0, 5.0]

    def test_subtract_self(self) -> None:
        # a - a = 0 (empty list, zero polynomial)
        p = [3.0, 1.0, 4.0]
        result = pn.subtract(p, p)
        assert result == []

    def test_subtract_zero(self) -> None:
        # a - 0 = a
        p = [1.0, 2.0, 3.0]
        assert pn.subtract(p, [0.0]) == p


# =========================================================================
# Multiply tests
# =========================================================================


class TestMultiply:
    """Tests for multiply(a, b) — polynomial convolution."""

    def test_multiply_basic(self) -> None:
        # (1 + 2x)(3 + 4x) = 3 + 4x + 6x + 8x² = 3 + 10x + 8x²
        result = pn.multiply([1.0, 2.0], [3.0, 4.0])
        assert result == [3.0, 10.0, 8.0]

    def test_multiply_by_one(self) -> None:
        # p * 1 = p
        p = [1.0, 2.0, 3.0]
        assert pn.multiply(p, [1.0]) == p

    def test_multiply_by_zero(self) -> None:
        # p * 0 = 0 (empty list)
        p = [1.0, 2.0, 3.0]
        result = pn.multiply(p, [0.0])
        assert result == []

    def test_multiply_commutativity(self) -> None:
        # multiply(a, b) == multiply(b, a)
        a = [1.0, 2.0]
        b = [3.0, 4.0, 5.0]
        assert pn.multiply(a, b) == pn.multiply(b, a)

    def test_multiply_degree(self) -> None:
        # Degree of a*b = degree(a) + degree(b)
        a = [1.0, 0.0, 1.0]  # degree 2
        b = [1.0, 1.0]       # degree 1
        result = pn.multiply(a, b)
        # result should be degree 3
        assert pn.degree(result) == 3


# =========================================================================
# Divmod / Divide / Modulo tests
# =========================================================================


class TestDivision:
    """Tests for divmod_poly, divide, modulo."""

    def test_divmod_exact(self) -> None:
        # (x^2 - 1) / (x - 1) = (x + 1) with remainder 0
        # x^2 - 1 = [-1.0, 0.0, 1.0], x - 1 = [-1.0, 1.0]
        quot, rem = pn.divmod_poly([-1.0, 0.0, 1.0], [-1.0, 1.0])
        # quotient is x + 1 = [1.0, 1.0]
        assert quot == [1.0, 1.0]
        assert rem == []

    def test_divmod_with_remainder(self) -> None:
        # Verify: dividend = divisor * quotient + remainder
        # [5, 1, 3, 2] / [2, 1] (from polynomial crate docs)
        dividend = [5.0, 1.0, 3.0, 2.0]
        divisor = [2.0, 1.0]
        quot, rem = pn.divmod_poly(dividend, divisor)
        # Reconstruct and verify
        reconstructed = pn.add(pn.multiply(divisor, quot), rem)
        for a, b in zip(reconstructed, dividend):
            assert abs(a - b) < 1e-9

    def test_divmod_zero_divisor_raises(self) -> None:
        # Division by zero polynomial must raise ValueError.
        with pytest.raises(ValueError):
            pn.divmod_poly([1.0, 2.0], [0.0])

    def test_divmod_empty_divisor_raises(self) -> None:
        # Empty list is also the zero polynomial.
        with pytest.raises(ValueError):
            pn.divmod_poly([1.0, 2.0], [])

    def test_divide_basic(self) -> None:
        # (x^2 - 1) / (x - 1) = (x + 1)
        result = pn.divide([-1.0, 0.0, 1.0], [-1.0, 1.0])
        assert result == [1.0, 1.0]

    def test_divide_by_zero_raises(self) -> None:
        with pytest.raises(ValueError):
            pn.divide([1.0, 2.0], [0.0])

    def test_modulo_basic(self) -> None:
        # x^2 mod x = 0 (x divides x^2 exactly)
        result = pn.modulo([0.0, 0.0, 1.0], [0.0, 1.0])
        assert result == []

    def test_modulo_with_remainder(self) -> None:
        # x^2 + 1 mod x = 1 (constant remainder)
        result = pn.modulo([1.0, 0.0, 1.0], [0.0, 1.0])
        assert result == [1.0]

    def test_modulo_by_zero_raises(self) -> None:
        with pytest.raises(ValueError):
            pn.modulo([1.0, 2.0], [0.0])


# =========================================================================
# Evaluate tests
# =========================================================================


class TestEvaluate:
    """Tests for evaluate(poly, x) — Horner's method evaluation."""

    def test_evaluate_constant(self) -> None:
        # p(x) = 7 → p(any) = 7
        assert pn.evaluate([7.0], 0.0) == 7.0
        assert pn.evaluate([7.0], 100.0) == 7.0

    def test_evaluate_at_zero(self) -> None:
        # p(0) is always the constant term (index 0).
        assert pn.evaluate([3.0, 2.0, 1.0], 0.0) == 3.0

    def test_evaluate_quadratic(self) -> None:
        # p(x) = 3 + 0x + x^2 → p(2) = 3 + 4 = 7
        assert pn.evaluate([3.0, 0.0, 1.0], 2.0) == 7.0

    def test_evaluate_linear(self) -> None:
        # p(x) = 1 + 2x → p(3) = 1 + 6 = 7
        assert pn.evaluate([1.0, 2.0], 3.0) == 7.0

    def test_evaluate_zero_poly(self) -> None:
        # The zero polynomial evaluates to 0 everywhere.
        assert pn.evaluate([], 5.0) == 0.0
        assert pn.evaluate([0.0], 999.0) == 0.0

    def test_evaluate_at_root(self) -> None:
        # x^2 - 1 has roots at x=1 and x=-1.
        # [-1.0, 0.0, 1.0] = -1 + x^2
        assert abs(pn.evaluate([-1.0, 0.0, 1.0], 1.0)) < 1e-9
        assert abs(pn.evaluate([-1.0, 0.0, 1.0], -1.0)) < 1e-9

    def test_evaluate_with_int_x(self) -> None:
        # x argument can be an integer (Python int).
        assert pn.evaluate([1.0, 1.0], 2) == 3.0


# =========================================================================
# GCD tests
# =========================================================================


class TestGcd:
    """Tests for gcd(a, b) — polynomial GCD via Euclidean algorithm."""

    def test_gcd_coprime_polynomials(self) -> None:
        # x and x+1 are coprime (no common factor), GCD = constant = [1.0]
        # gcd([0,1], [1,1]) should be a unit (constant polynomial)
        result = pn.gcd([0.0, 1.0], [1.0, 1.0])
        # The result is a non-zero constant (monic or scalar multiple of 1)
        assert len(result) == 1
        assert abs(result[0]) > 1e-9

    def test_gcd_common_factor(self) -> None:
        # gcd(x^2 - 3x + 2, x - 1) = x - 1 (since x^2 - 3x + 2 = (x-1)(x-2))
        # x^2 - 3x + 2 = [2, -3, 1], x - 1 = [-1, 1]
        result = pn.gcd([2.0, -3.0, 1.0], [-1.0, 1.0])
        # The GCD should be a scalar multiple of (x - 1) = [-1, 1]
        # It's normalized, so the leading coefficient is whatever polynomial divides both
        assert pn.degree(result) == 1

    def test_gcd_with_self(self) -> None:
        # gcd(p, p) = p (or a scalar multiple of p, normalized)
        p = [1.0, 2.0, 1.0]  # (x+1)^2
        result = pn.gcd(p, p)
        # Should be a non-zero polynomial with the same degree as p
        assert pn.degree(result) == pn.degree(p)

    def test_gcd_result_divides_both(self) -> None:
        # The GCD must divide both inputs with zero remainder.
        a = [2.0, -3.0, 1.0]  # x^2 - 3x + 2 = (x-1)(x-2)
        b = [2.0, -1.0, -1.0]  # x^2 - x - 2  ... wait, let's use something cleaner
        # Use a = (x-1)(x-2), b = (x-1)(x-3) to get gcd = (x-1)
        # a = x^2 - 3x + 2 = [2, -3, 1]
        # b = x^2 - 4x + 3 = [3, -4, 1]
        a = [2.0, -3.0, 1.0]
        b = [3.0, -4.0, 1.0]
        g = pn.gcd(a, b)
        # g should divide a with zero remainder
        rem_a = pn.modulo(a, g)
        rem_b = pn.modulo(b, g)
        # Each remainder should be empty (or all-zero)
        for coeff in rem_a:
            assert abs(coeff) < 1e-6
        for coeff in rem_b:
            assert abs(coeff) < 1e-6

    def test_gcd_with_zero(self) -> None:
        # gcd(p, 0) = p (zero polynomial is divisible by everything)
        p = [1.0, 1.0]
        result = pn.gcd(p, [0.0])
        # Should be a polynomial of the same degree as p
        assert pn.degree(result) == pn.degree(p)


# =========================================================================
# Error handling tests
# =========================================================================


class TestErrors:
    """Tests for error conditions and type validation."""

    def test_non_list_argument_raises(self) -> None:
        # Passing a non-list raises ValueError.
        with pytest.raises((ValueError, TypeError)):
            pn.normalize((1.0, 2.0))  # tuple, not list

    def test_mixed_type_list_works(self) -> None:
        # A list mixing int and float is accepted (ints are auto-converted).
        result = pn.normalize([1, 0, 0])
        assert result == [1.0]

    def test_divmod_type_error(self) -> None:
        # Passing a string instead of a list raises an error.
        with pytest.raises((ValueError, TypeError)):
            pn.divmod_poly("hello", [1.0])

    def test_evaluate_returns_float(self) -> None:
        # evaluate always returns a Python float.
        result = pn.evaluate([1.0, 1.0], 1.0)
        assert isinstance(result, float)

    def test_divmod_returns_tuple(self) -> None:
        # divmod_poly must return a 2-tuple.
        result = pn.divmod_poly([1.0, 1.0], [1.0])
        assert isinstance(result, tuple)
        assert len(result) == 2
