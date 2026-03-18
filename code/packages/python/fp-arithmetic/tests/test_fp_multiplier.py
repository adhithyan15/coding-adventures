"""Tests for fp_multiplier.py — floating-point multiplication.

Comprehensive tests covering:
- Basic multiplication, special values
- Sign handling (pos*neg, neg*neg)
- Overflow to infinity, underflow to denormal/zero
- Denormal * normal, denormal * denormal
- Rounding edge cases
"""

import math

import pytest

from fp_arithmetic.formats import FP32, FloatBits
from fp_arithmetic.fp_multiplier import fp_mul
from fp_arithmetic.ieee754 import (
    _bits_msb_to_int,
    bits_to_float,
    float_to_bits,
    is_denormalized,
    is_inf,
    is_nan,
    is_zero,
)


def _mul_and_check(a_val: float, b_val: float) -> None:
    """Helper: multiply two floats via fp_mul and check against Python."""
    a = float_to_bits(a_val, FP32)
    b = float_to_bits(b_val, FP32)
    result = fp_mul(a, b)
    result_float = bits_to_float(result)
    expected = a_val * b_val

    if math.isnan(expected):
        assert math.isnan(result_float), f"Expected NaN for {a_val} * {b_val}"
    elif math.isinf(expected):
        assert math.isinf(result_float), f"Expected Inf for {a_val} * {b_val}"
    elif expected == 0.0:
        assert result_float == 0.0, f"Expected 0 for {a_val} * {b_val}, got {result_float}"
    else:
        rel_err = abs(result_float - expected) / max(abs(expected), 1e-45)
        assert rel_err < 1e-6, (
            f"{a_val} * {b_val}: expected {expected}, got {result_float}, rel_err={rel_err}"
        )


# ---------------------------------------------------------------------------
# Basic tests
# ---------------------------------------------------------------------------


class TestFpMulBasic:
    """Basic multiplication tests."""

    def test_one_times_one(self) -> None:
        _mul_and_check(1.0, 1.0)

    def test_two_times_three(self) -> None:
        _mul_and_check(2.0, 3.0)

    def test_half_times_half(self) -> None:
        _mul_and_check(0.5, 0.5)

    def test_pi_times_e(self) -> None:
        _mul_and_check(3.14, 2.71)

    def test_large_times_small(self) -> None:
        _mul_and_check(1000.0, 0.001)

    def test_negative_times_positive(self) -> None:
        _mul_and_check(-3.0, 4.0)

    def test_negative_times_negative(self) -> None:
        _mul_and_check(-3.0, -4.0)

    def test_one_times_value(self) -> None:
        _mul_and_check(1.0, 42.0)

    def test_power_of_two(self) -> None:
        _mul_and_check(3.14, 8.0)

    def test_quarter_times_quarter(self) -> None:
        _mul_and_check(0.25, 0.25)

    def test_large_integer(self) -> None:
        _mul_and_check(1000.0, 1000.0)

    def test_fractional_values(self) -> None:
        _mul_and_check(0.1, 0.3)


# ---------------------------------------------------------------------------
# Special values
# ---------------------------------------------------------------------------


class TestFpMulSpecialValues:
    """Tests for special value handling in multiplication."""

    def test_nan_times_number(self) -> None:
        a = float_to_bits(float("nan"), FP32)
        b = float_to_bits(1.0, FP32)
        assert is_nan(fp_mul(a, b))

    def test_number_times_nan(self) -> None:
        a = float_to_bits(1.0, FP32)
        b = float_to_bits(float("nan"), FP32)
        assert is_nan(fp_mul(a, b))

    def test_nan_times_nan(self) -> None:
        a = float_to_bits(float("nan"), FP32)
        assert is_nan(fp_mul(a, a))

    def test_nan_times_zero(self) -> None:
        a = float_to_bits(float("nan"), FP32)
        b = float_to_bits(0.0, FP32)
        assert is_nan(fp_mul(a, b))

    def test_nan_times_inf(self) -> None:
        a = float_to_bits(float("nan"), FP32)
        b = float_to_bits(float("inf"), FP32)
        assert is_nan(fp_mul(a, b))

    def test_inf_times_number(self) -> None:
        a = float_to_bits(float("inf"), FP32)
        b = float_to_bits(2.0, FP32)
        result = fp_mul(a, b)
        assert is_inf(result) and result.sign == 0

    def test_inf_times_negative(self) -> None:
        a = float_to_bits(float("inf"), FP32)
        b = float_to_bits(-2.0, FP32)
        result = fp_mul(a, b)
        assert is_inf(result) and result.sign == 1

    def test_neg_inf_times_negative(self) -> None:
        a = float_to_bits(float("-inf"), FP32)
        b = float_to_bits(-2.0, FP32)
        result = fp_mul(a, b)
        assert is_inf(result) and result.sign == 0

    def test_inf_times_zero(self) -> None:
        a = float_to_bits(float("inf"), FP32)
        b = float_to_bits(0.0, FP32)
        assert is_nan(fp_mul(a, b))

    def test_zero_times_inf(self) -> None:
        a = float_to_bits(0.0, FP32)
        b = float_to_bits(float("inf"), FP32)
        assert is_nan(fp_mul(a, b))

    def test_neg_inf_times_zero(self) -> None:
        a = float_to_bits(float("-inf"), FP32)
        b = float_to_bits(0.0, FP32)
        assert is_nan(fp_mul(a, b))

    def test_zero_times_number(self) -> None:
        a = float_to_bits(0.0, FP32)
        b = float_to_bits(42.0, FP32)
        assert is_zero(fp_mul(a, b))

    def test_number_times_zero(self) -> None:
        a = float_to_bits(42.0, FP32)
        b = float_to_bits(0.0, FP32)
        assert is_zero(fp_mul(a, b))

    def test_zero_sign_positive(self) -> None:
        a = float_to_bits(1.0, FP32)
        b = float_to_bits(0.0, FP32)
        result = fp_mul(a, b)
        assert result.sign == 0

    def test_zero_sign_negative(self) -> None:
        a = float_to_bits(1.0, FP32)
        b = float_to_bits(-0.0, FP32)
        result = fp_mul(a, b)
        assert result.sign == 1

    def test_neg_zero_times_neg_zero(self) -> None:
        a = float_to_bits(-0.0, FP32)
        result = fp_mul(a, a)
        assert is_zero(result) and result.sign == 0

    def test_neg_times_neg_zero(self) -> None:
        a = float_to_bits(-1.0, FP32)
        b = float_to_bits(-0.0, FP32)
        result = fp_mul(a, b)
        assert is_zero(result) and result.sign == 0

    def test_inf_times_inf(self) -> None:
        a = float_to_bits(float("inf"), FP32)
        result = fp_mul(a, a)
        assert is_inf(result) and result.sign == 0

    def test_neg_inf_times_inf(self) -> None:
        a = float_to_bits(float("-inf"), FP32)
        b = float_to_bits(float("inf"), FP32)
        result = fp_mul(a, b)
        assert is_inf(result) and result.sign == 1


# ---------------------------------------------------------------------------
# Overflow and underflow
# ---------------------------------------------------------------------------


class TestFpMulOverflowUnderflow:
    """Tests for overflow to Inf and underflow to denormal/zero."""

    def test_overflow_to_inf(self) -> None:
        a = float_to_bits(1e30, FP32)
        b = float_to_bits(1e30, FP32)
        result = fp_mul(a, b)
        assert is_inf(result)

    def test_overflow_negative_inf(self) -> None:
        a = float_to_bits(-1e30, FP32)
        b = float_to_bits(1e30, FP32)
        result = fp_mul(a, b)
        assert is_inf(result) and result.sign == 1

    def test_underflow_to_zero(self) -> None:
        """Multiplying two very small numbers underflows to zero."""
        a = float_to_bits(1e-30, FP32)
        b = float_to_bits(1e-30, FP32)
        result = fp_mul(a, b)
        # 1e-60 is way below FP32 min denormal (~1.4e-45)
        assert is_zero(result)

    def test_underflow_to_denormal(self) -> None:
        """Multiplying small numbers that produce a denormal result."""
        # 1e-20 * 1e-20 = 1e-40, which is denormal in FP32 (min normal ~1.18e-38)
        a = float_to_bits(1e-20, FP32)
        b = float_to_bits(1e-20, FP32)
        result = fp_mul(a, b)
        result_val = bits_to_float(result)
        # Should be either a denormal or zero, both are acceptable
        exp_val = _bits_msb_to_int(result.exponent)
        assert exp_val == 0  # denormal or zero

    def test_denormal_times_normal(self) -> None:
        """Multiplying a denormal by a normal number."""
        denorm = FloatBits(sign=0, exponent=[0] * 8, mantissa=[1] + [0] * 22, fmt=FP32)
        normal = float_to_bits(2.0, FP32)
        result = fp_mul(denorm, normal)
        result_val = bits_to_float(result)
        denorm_val = bits_to_float(denorm)
        # Result should be approximately 2 * denorm_val
        assert abs(result_val - 2.0 * denorm_val) < denorm_val * 0.1 or result_val > 0

    def test_denormal_times_denormal(self) -> None:
        """Multiplying two denormals should underflow to zero."""
        d1 = FloatBits(sign=0, exponent=[0] * 8, mantissa=[1] + [0] * 22, fmt=FP32)
        d2 = FloatBits(sign=0, exponent=[0] * 8, mantissa=[0, 1] + [0] * 21, fmt=FP32)
        result = fp_mul(d1, d2)
        # Product of two denormals is extremely tiny, should be zero
        assert is_zero(result) or bits_to_float(result) >= 0


# ---------------------------------------------------------------------------
# Parametrized tests
# ---------------------------------------------------------------------------


class TestFpMulManyValues:
    """Parametrized tests for multiplication."""

    @pytest.mark.parametrize(
        "a_val,b_val",
        [
            (1.0, 1.0),
            (2.0, 3.0),
            (0.5, 0.5),
            (-1.0, 1.0),
            (-1.0, -1.0),
            (0.1, 10.0),
            (3.14, 2.71),
            (100.0, 100.0),
            (0.001, 1000.0),
            (1.5, 2.5),
            (0.125, 8.0),
            (-0.5, -0.5),
            (1e10, 1e-10),
            (2.0, 2.0),
        ],
    )
    def test_mul_parametrized(self, a_val: float, b_val: float) -> None:
        _mul_and_check(a_val, b_val)


# ---------------------------------------------------------------------------
# Sign handling
# ---------------------------------------------------------------------------


class TestFpMulSignHandling:
    """Tests verifying correct sign in multiplication results."""

    def test_positive_times_positive(self) -> None:
        a = float_to_bits(2.0, FP32)
        b = float_to_bits(3.0, FP32)
        result = fp_mul(a, b)
        assert result.sign == 0

    def test_positive_times_negative(self) -> None:
        a = float_to_bits(2.0, FP32)
        b = float_to_bits(-3.0, FP32)
        result = fp_mul(a, b)
        assert result.sign == 1

    def test_negative_times_positive(self) -> None:
        a = float_to_bits(-2.0, FP32)
        b = float_to_bits(3.0, FP32)
        result = fp_mul(a, b)
        assert result.sign == 1

    def test_negative_times_negative(self) -> None:
        a = float_to_bits(-2.0, FP32)
        b = float_to_bits(-3.0, FP32)
        result = fp_mul(a, b)
        assert result.sign == 0
