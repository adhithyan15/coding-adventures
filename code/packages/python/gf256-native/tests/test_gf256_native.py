"""
Comprehensive test suite for the gf256_native extension.

GF(2^8) — Galois Field with 256 elements. Each element is a byte (0-255).
Arithmetic rules in GF(256):
  - Add = XOR (no carries, since 1+1=0 in characteristic 2)
  - Subtract = XOR (same as add)
  - Multiply uses log/antilog lookup tables
  - Divide uses inverse lookup
  - power(2, 255) == 1 (Fermat's little theorem for finite fields)

Tests cover all 6 functions plus module constants.
"""

import pytest

import gf256_native as gf


# =========================================================================
# Module constants tests
# =========================================================================


class TestConstants:
    """Tests for ZERO, ONE, and PRIMITIVE_POLYNOMIAL constants."""

    def test_zero_constant(self) -> None:
        assert gf.ZERO == 0

    def test_one_constant(self) -> None:
        assert gf.ONE == 1

    def test_primitive_polynomial(self) -> None:
        # The primitive polynomial is x^8 + x^4 + x^3 + x^2 + 1 = 0x11D = 285
        assert gf.PRIMITIVE_POLYNOMIAL == 0x11D
        assert gf.PRIMITIVE_POLYNOMIAL == 285

    def test_constants_are_integers(self) -> None:
        assert isinstance(gf.ZERO, int)
        assert isinstance(gf.ONE, int)
        assert isinstance(gf.PRIMITIVE_POLYNOMIAL, int)


# =========================================================================
# Add tests
# =========================================================================


class TestAdd:
    """Tests for add(a, b) — XOR addition in GF(256)."""

    def test_add_known_value(self) -> None:
        # From the GF(256) documentation: 0x53 XOR 0xCA = 0x99
        assert gf.add(0x53, 0xCA) == (0x53 ^ 0xCA)
        assert gf.add(0x53, 0xCA) == 0x99

    def test_add_with_zero(self) -> None:
        # x + 0 = x for all x (0 is the additive identity)
        for x in [0, 1, 127, 128, 255]:
            assert gf.add(x, 0) == x
            assert gf.add(0, x) == x

    def test_add_self_is_zero(self) -> None:
        # x + x = 0 in GF(256) (every element is its own additive inverse)
        for x in [0, 1, 2, 127, 255]:
            assert gf.add(x, x) == 0

    def test_add_commutativity(self) -> None:
        # add(a, b) == add(b, a)
        assert gf.add(0x12, 0x34) == gf.add(0x34, 0x12)
        assert gf.add(0xAB, 0xCD) == gf.add(0xCD, 0xAB)

    def test_add_associativity(self) -> None:
        # add(add(a, b), c) == add(a, add(b, c))
        a, b, c = 0x12, 0x34, 0x56
        assert gf.add(gf.add(a, b), c) == gf.add(a, gf.add(b, c))

    def test_add_returns_int_in_range(self) -> None:
        # Result is always a byte (0-255)
        result = gf.add(255, 255)
        assert isinstance(result, int)
        assert 0 <= result <= 255


# =========================================================================
# Subtract tests
# =========================================================================


class TestSubtract:
    """Tests for subtract(a, b) — equals add in GF(256)."""

    def test_subtract_equals_add(self) -> None:
        # In characteristic 2, a - b = a + b = a XOR b
        for a, b in [(0x12, 0x34), (0xFF, 0x01), (0x53, 0xCA), (0, 0)]:
            assert gf.subtract(a, b) == gf.add(a, b)

    def test_subtract_self_is_zero(self) -> None:
        # a - a = 0 in any field
        for x in [0, 1, 127, 255]:
            assert gf.subtract(x, x) == 0

    def test_subtract_zero(self) -> None:
        # a - 0 = a
        for x in [0, 1, 128, 255]:
            assert gf.subtract(x, 0) == x


# =========================================================================
# Multiply tests
# =========================================================================


class TestMultiply:
    """Tests for multiply(a, b) — GF(256) field multiplication."""

    def test_multiply_by_zero(self) -> None:
        # 0 * x = x * 0 = 0 for all x
        for x in [0, 1, 2, 127, 255]:
            assert gf.multiply(x, 0) == 0
            assert gf.multiply(0, x) == 0

    def test_multiply_by_one(self) -> None:
        # 1 * x = x * 1 = x (1 is the multiplicative identity)
        for x in [0, 1, 2, 127, 255]:
            assert gf.multiply(x, 1) == x
            assert gf.multiply(1, x) == x

    def test_multiply_commutativity(self) -> None:
        # multiply(a, b) == multiply(b, a)
        assert gf.multiply(3, 7) == gf.multiply(7, 3)
        assert gf.multiply(0x57, 0x83) == gf.multiply(0x83, 0x57)

    def test_multiply_known_values(self) -> None:
        # 2 * 2 = 4 in GF(256) (no reduction needed, 4 < 256)
        assert gf.multiply(2, 2) == 4
        # 2 * 128 = 256 → reduced mod 0x11D = 0x1D = 29
        assert gf.multiply(2, 128) == 29

    def test_multiply_associativity(self) -> None:
        # multiply(multiply(a, b), c) == multiply(a, multiply(b, c))
        a, b, c = 3, 7, 11
        assert gf.multiply(gf.multiply(a, b), c) == gf.multiply(a, gf.multiply(b, c))

    def test_multiply_distributivity(self) -> None:
        # a * (b + c) == (a*b) + (a*c)  (distributive law)
        a, b, c = 0x57, 0x13, 0x42
        lhs = gf.multiply(a, gf.add(b, c))
        rhs = gf.add(gf.multiply(a, b), gf.multiply(a, c))
        assert lhs == rhs

    def test_multiply_result_in_range(self) -> None:
        # Result is always a byte (0-255)
        for a in [0, 1, 255, 127, 128]:
            for b in [0, 1, 255, 127]:
                result = gf.multiply(a, b)
                assert isinstance(result, int)
                assert 0 <= result <= 255


# =========================================================================
# Divide tests
# =========================================================================


class TestDivide:
    """Tests for divide(a, b) — GF(256) field division."""

    def test_divide_by_one(self) -> None:
        # a / 1 = a for all a
        for x in [0, 1, 2, 127, 255]:
            assert gf.divide(x, 1) == x

    def test_divide_zero_by_anything(self) -> None:
        # 0 / b = 0 for all non-zero b
        for b in [1, 2, 127, 255]:
            assert gf.divide(0, b) == 0

    def test_divide_self(self) -> None:
        # a / a = 1 for all non-zero a
        for a in [1, 2, 3, 127, 255]:
            assert gf.divide(a, a) == 1

    def test_divide_by_zero_raises(self) -> None:
        # Division by zero must raise ValueError.
        with pytest.raises(ValueError):
            gf.divide(1, 0)

    def test_divide_various_by_zero_raises(self) -> None:
        # Any numerator / 0 raises ValueError.
        with pytest.raises(ValueError):
            gf.divide(255, 0)
        with pytest.raises(ValueError):
            gf.divide(0, 0)

    def test_divide_inverse_of_multiply(self) -> None:
        # divide(multiply(a, b), b) == a for non-zero b
        for a in [1, 2, 5, 127]:
            for b in [1, 3, 7, 255]:
                assert gf.divide(gf.multiply(a, b), b) == a


# =========================================================================
# Power tests
# =========================================================================


class TestPower:
    """Tests for power(base, exp) — GF(256) exponentiation."""

    def test_power_zero_exp(self) -> None:
        # x^0 = 1 for all non-zero x (and 0^0 = 1 by convention)
        for x in [0, 1, 2, 127, 255]:
            assert gf.power(x, 0) == 1

    def test_power_one_exp(self) -> None:
        # x^1 = x for all x
        for x in [0, 1, 2, 127, 255]:
            assert gf.power(x, 1) == x

    def test_power_zero_base(self) -> None:
        # 0^n = 0 for n > 0
        for n in [1, 2, 10, 255]:
            assert gf.power(0, n) == 0

    def test_power_generator_order(self) -> None:
        # The generator g=2 has order 255: 2^255 = 1 in GF(256)
        # This is Fermat's little theorem for finite fields.
        assert gf.power(2, 255) == 1

    def test_power_squaring(self) -> None:
        # 2^1 = 2, 2^2 = 4, 2^3 = 8, ... up to 2^7 = 128 (no overflow yet)
        assert gf.power(2, 1) == 2
        assert gf.power(2, 2) == 4
        assert gf.power(2, 3) == 8
        assert gf.power(2, 4) == 16
        assert gf.power(2, 7) == 128

    def test_power_consistency_with_multiply(self) -> None:
        # power(x, 3) == multiply(multiply(x, x), x)
        for x in [2, 3, 5, 7]:
            p3 = gf.power(x, 3)
            m3 = gf.multiply(gf.multiply(x, x), x)
            assert p3 == m3

    def test_power_8_first_reduction(self) -> None:
        # 2^8 should be 29 (0x1D): the first time reduction occurs.
        # 128 << 1 = 256; 256 XOR 0x11D = 0x01D = 29
        assert gf.power(2, 8) == 29


# =========================================================================
# Inverse tests
# =========================================================================


class TestInverse:
    """Tests for inverse(a) — multiplicative inverse in GF(256)."""

    def test_inverse_one(self) -> None:
        # inverse(1) == 1 (1 is its own inverse)
        assert gf.inverse(1) == 1

    def test_inverse_zero_raises(self) -> None:
        # Zero has no multiplicative inverse.
        with pytest.raises(ValueError):
            gf.inverse(0)

    def test_inverse_satisfies_definition(self) -> None:
        # a * inverse(a) == 1 for all non-zero a
        for a in [1, 2, 3, 7, 127, 128, 255]:
            assert gf.multiply(a, gf.inverse(a)) == 1

    def test_inverse_of_inverse(self) -> None:
        # inverse(inverse(a)) == a for all non-zero a
        for a in [1, 2, 5, 127, 255]:
            assert gf.inverse(gf.inverse(a)) == a

    def test_inverse_255(self) -> None:
        # 255 is its own inverse if multiply(255, 255) == 1
        result = gf.multiply(255, gf.inverse(255))
        assert result == 1


# =========================================================================
# Range validation tests
# =========================================================================


class TestRangeValidation:
    """Tests that out-of-range arguments raise ValueError."""

    def test_add_out_of_range_raises(self) -> None:
        with pytest.raises(ValueError):
            gf.add(256, 1)
        with pytest.raises(ValueError):
            gf.add(-1, 1)
        with pytest.raises(ValueError):
            gf.add(1, 256)

    def test_multiply_out_of_range_raises(self) -> None:
        with pytest.raises(ValueError):
            gf.multiply(256, 1)
        with pytest.raises(ValueError):
            gf.multiply(1, -5)

    def test_inverse_out_of_range_raises(self) -> None:
        with pytest.raises(ValueError):
            gf.inverse(256)
        with pytest.raises(ValueError):
            gf.inverse(-1)

    def test_power_base_out_of_range_raises(self) -> None:
        with pytest.raises(ValueError):
            gf.power(256, 1)
        with pytest.raises(ValueError):
            gf.power(-1, 2)

    def test_power_negative_exp_raises(self) -> None:
        with pytest.raises(ValueError):
            gf.power(2, -1)
