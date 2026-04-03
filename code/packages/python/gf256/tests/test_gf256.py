"""Tests for the gf256 package."""

import pytest
from gf256 import (
    VERSION,
    ZERO,
    ONE,
    PRIMITIVE_POLYNOMIAL,
    LOG,
    ALOG,
    add,
    subtract,
    multiply,
    divide,
    power,
    inverse,
    zero,
    one,
)


# =============================================================================
# VERSION
# =============================================================================


class TestVersion:
    def test_is_semver(self):
        parts = VERSION.split(".")
        assert len(parts) == 3
        assert all(p.isdigit() for p in parts)


# =============================================================================
# Constants
# =============================================================================


class TestConstants:
    def test_zero_is_0(self):
        assert ZERO == 0

    def test_one_is_1(self):
        assert ONE == 1

    def test_primitive_polynomial(self):
        assert PRIMITIVE_POLYNOMIAL == 0x11D
        assert PRIMITIVE_POLYNOMIAL == 285


# =============================================================================
# Log/Antilog Tables
# =============================================================================


class TestTables:
    def test_alog_has_256_entries(self):
        assert len(ALOG) == 256  # indices 0..254 + ALOG[255]=1

    def test_log_has_256_entries(self):
        assert len(LOG) == 256

    def test_alog_0_is_1(self):
        assert ALOG[0] == 1

    def test_alog_1_is_2(self):
        assert ALOG[1] == 2

    def test_alog_8_is_29(self):
        # 2^8 = 256; 256 XOR 0x11D = 0x1D = 29
        assert ALOG[8] == 29

    def test_alog_values_in_range(self):
        for i in range(255):
            assert 1 <= ALOG[i] <= 255

    def test_alog_is_bijection(self):
        # ALOG[0..254] are all distinct non-zero values
        assert len(set(ALOG[:255])) == 255
        assert 0 not in ALOG[:255]

    def test_alog_log_roundtrip(self):
        for x in range(1, 256):
            assert ALOG[LOG[x]] == x

    def test_log_alog_roundtrip(self):
        for i in range(255):
            assert LOG[ALOG[i]] == i

    def test_log_1_is_0(self):
        assert LOG[1] == 0

    def test_log_2_is_1(self):
        assert LOG[2] == 1


# =============================================================================
# add
# =============================================================================


class TestAdd:
    def test_add_zero_is_identity(self):
        for x in range(256):
            assert add(0, x) == x
            assert add(x, 0) == x

    def test_add_self_is_zero(self):
        for x in range(256):
            assert add(x, x) == 0

    def test_commutative(self):
        for x in range(32):
            for y in range(32):
                assert add(x, y) == add(y, x)

    def test_associative(self):
        a, b, c = 0x53, 0xCA, 0x7F
        assert add(add(a, b), c) == add(a, add(b, c))

    def test_is_xor(self):
        for x in range(256):
            assert add(x, 0x42) == x ^ 0x42


# =============================================================================
# subtract
# =============================================================================


class TestSubtract:
    def test_subtract_self_is_zero(self):
        for x in range(256):
            assert subtract(x, x) == 0

    def test_equals_add_in_char2(self):
        for x in range(32):
            for y in range(32):
                assert subtract(x, y) == add(x, y)

    def test_subtract_zero_is_identity(self):
        for x in range(256):
            assert subtract(0, x) == x


# =============================================================================
# multiply
# =============================================================================


class TestMultiply:
    def test_multiply_by_zero(self):
        for x in range(256):
            assert multiply(x, 0) == 0
            assert multiply(0, x) == 0

    def test_multiply_by_one(self):
        for x in range(256):
            assert multiply(x, 1) == x
            assert multiply(1, x) == x

    def test_commutative(self):
        for x in range(32):
            for y in range(32):
                assert multiply(x, y) == multiply(y, x)

    def test_associative(self):
        a, b, c = 0x53, 0xCA, 0x3D
        assert multiply(multiply(a, b), c) == multiply(a, multiply(b, c))

    def test_spot_check_0x53_times_0x8C_equals_1(self):
        # With 0x11D polynomial: inverse(0x53) = 0x8C
        assert multiply(0x53, 0x8C) == 0x01

    def test_distributive_over_add(self):
        a, b, c = 0x34, 0x56, 0x78
        assert multiply(a, add(b, c)) == add(multiply(a, b), multiply(a, c))


# =============================================================================
# divide
# =============================================================================


class TestDivide:
    def test_divide_by_zero_raises(self):
        with pytest.raises(ValueError):
            divide(1, 0)
        with pytest.raises(ValueError):
            divide(0, 0)

    def test_divide_by_one(self):
        for x in range(256):
            assert divide(x, 1) == x

    def test_divide_zero_by_anything(self):
        for x in range(1, 256):
            assert divide(0, x) == 0

    def test_divide_self_is_one(self):
        for x in range(1, 256):
            assert divide(x, x) == 1

    def test_divide_is_inverse_of_multiply(self):
        for a in range(0, 32):
            for b in range(1, 32):
                assert divide(multiply(a, b), b) == a


# =============================================================================
# power
# =============================================================================


class TestPower:
    def test_any_nonzero_to_zero_is_one(self):
        for x in range(1, 256):
            assert power(x, 0) == 1

    def test_zero_to_zero_is_one(self):
        assert power(0, 0) == 1

    def test_zero_to_positive_is_zero(self):
        assert power(0, 1) == 0
        assert power(0, 5) == 0

    def test_any_to_one_is_self(self):
        for x in range(256):
            assert power(x, 1) == x

    def test_generator_order_255(self):
        # g^255 = 1 (the multiplicative group has order 255)
        assert power(2, 255) == 1

    def test_power_matches_alog(self):
        for i in range(255):
            assert power(2, i) == ALOG[i]

    def test_fermat_x254_is_inverse(self):
        # x^254 = x^(-1) since x^255 = 1
        for x in range(1, 21):
            assert power(x, 254) == inverse(x)


# =============================================================================
# inverse
# =============================================================================


class TestInverse:
    def test_inverse_zero_raises(self):
        with pytest.raises(ValueError):
            inverse(0)

    def test_inverse_one_is_one(self):
        assert inverse(1) == 1

    def test_inverse_times_self_is_one_range(self):
        for x in range(1, 11):
            assert multiply(x, inverse(x)) == 1

    def test_inverse_times_self_is_one_all(self):
        for x in range(1, 256):
            assert multiply(x, inverse(x)) == 1

    def test_inverse_of_inverse_is_self(self):
        for x in range(1, 256):
            assert inverse(inverse(x)) == x

    def test_spot_check_0x53_inverse_is_0x8C(self):
        # With primitive polynomial 0x11D: inverse(0x53) = 0x8C
        assert inverse(0x53) == 0x8C
        assert multiply(0x53, inverse(0x53)) == 1


# =============================================================================
# zero and one
# =============================================================================


class TestZeroAndOne:
    def test_zero_returns_0(self):
        assert zero() == 0

    def test_one_returns_1(self):
        assert one() == 1

    def test_zero_is_additive_identity(self):
        assert add(zero(), 0x42) == 0x42
        assert add(0x42, zero()) == 0x42

    def test_one_is_multiplicative_identity(self):
        assert multiply(one(), 0x42) == 0x42
        assert multiply(0x42, one()) == 0x42
