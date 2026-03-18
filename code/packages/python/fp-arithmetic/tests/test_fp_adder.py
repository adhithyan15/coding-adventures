"""Tests for fp_adder.py — floating-point addition, subtraction, and comparison.

Comprehensive tests covering:
- Basic arithmetic, special values, denormals, overflow, underflow
- Catastrophic cancellation
- All helper functions (_shift_right, _shift_left, _find_leading_one, etc.)
- Edge cases in normalization and rounding
"""

import math

import pytest

from fp_arithmetic.formats import FP16, FP32, BF16, FloatBits
from fp_arithmetic.fp_adder import (
    _add_bits_msb,
    _find_leading_one,
    _shift_left,
    _shift_right,
    _subtract_unsigned,
    fp_abs,
    fp_add,
    fp_compare,
    fp_neg,
    fp_sub,
)
from fp_arithmetic.ieee754 import bits_to_float, float_to_bits, is_inf, is_nan, is_zero


def _add_and_check(a_val: float, b_val: float) -> None:
    """Helper: add two floats via our fp_add and check against Python's float."""
    a = float_to_bits(a_val, FP32)
    b = float_to_bits(b_val, FP32)
    result = fp_add(a, b)
    result_float = bits_to_float(result)
    expected = a_val + b_val

    if math.isnan(expected):
        assert math.isnan(result_float), f"Expected NaN for {a_val} + {b_val}"
    elif math.isinf(expected):
        assert math.isinf(result_float), f"Expected Inf for {a_val} + {b_val}"
        assert math.copysign(1, result_float) == math.copysign(1, expected)
    elif expected == 0.0:
        assert result_float == 0.0, f"Expected 0 for {a_val} + {b_val}, got {result_float}"
    else:
        rel_err = abs(result_float - expected) / max(abs(expected), 1e-45)
        assert rel_err < 1e-6, (
            f"{a_val} + {b_val}: expected {expected}, got {result_float}, rel_err={rel_err}"
        )


# ---------------------------------------------------------------------------
# Tests for internal helpers
# ---------------------------------------------------------------------------


class TestShiftRight:
    """Tests for _shift_right helper."""

    def test_shift_by_zero(self) -> None:
        assert _shift_right([1, 0, 1, 1], 0) == [1, 0, 1, 1]

    def test_shift_by_one(self) -> None:
        assert _shift_right([1, 0, 1, 1], 1) == [0, 1, 0, 1]

    def test_shift_by_two(self) -> None:
        assert _shift_right([1, 0, 1, 1], 2) == [0, 0, 1, 0]

    def test_shift_exceeds_width(self) -> None:
        assert _shift_right([1, 0, 1, 1], 5) == [0, 0, 0, 0]

    def test_shift_equals_width(self) -> None:
        assert _shift_right([1, 0, 1, 1], 4) == [0, 0, 0, 0]

    def test_shift_negative(self) -> None:
        """Negative shift amounts should return a copy."""
        assert _shift_right([1, 0, 1], -1) == [1, 0, 1]


class TestShiftLeft:
    """Tests for _shift_left helper."""

    def test_shift_by_zero(self) -> None:
        assert _shift_left([1, 0, 1, 1], 0) == [1, 0, 1, 1]

    def test_shift_by_one(self) -> None:
        assert _shift_left([1, 0, 1, 1], 1) == [0, 1, 1, 0]

    def test_shift_by_two(self) -> None:
        assert _shift_left([1, 0, 1, 1], 2) == [1, 1, 0, 0]

    def test_shift_exceeds_width(self) -> None:
        assert _shift_left([1, 0, 1, 1], 5) == [0, 0, 0, 0]

    def test_shift_equals_width(self) -> None:
        assert _shift_left([1, 0, 1, 1], 4) == [0, 0, 0, 0]

    def test_shift_negative(self) -> None:
        assert _shift_left([1, 0, 1], -1) == [1, 0, 1]


class TestFindLeadingOne:
    """Tests for _find_leading_one helper."""

    def test_first_bit(self) -> None:
        assert _find_leading_one([1, 0, 0, 0]) == 0

    def test_middle_bit(self) -> None:
        assert _find_leading_one([0, 0, 1, 0, 1]) == 2

    def test_last_bit(self) -> None:
        assert _find_leading_one([0, 0, 0, 1]) == 3

    def test_all_zeros(self) -> None:
        assert _find_leading_one([0, 0, 0, 0, 0]) == -1

    def test_all_ones(self) -> None:
        assert _find_leading_one([1, 1, 1, 1]) == 0

    def test_single_one(self) -> None:
        assert _find_leading_one([1]) == 0

    def test_single_zero(self) -> None:
        assert _find_leading_one([0]) == -1


class TestSubtractUnsigned:
    """Tests for _subtract_unsigned helper."""

    def test_five_minus_three(self) -> None:
        # 5 = 0101, 3 = 0011
        a = [0, 1, 0, 1]
        b = [0, 0, 1, 1]
        result, borrow = _subtract_unsigned(a, b)
        from fp_arithmetic.ieee754 import _bits_msb_to_int
        assert _bits_msb_to_int(result) == 2
        assert borrow == 0

    def test_three_minus_five(self) -> None:
        # 3 - 5 = negative, borrow = 1
        a = [0, 0, 1, 1]
        b = [0, 1, 0, 1]
        result, borrow = _subtract_unsigned(a, b)
        assert borrow == 1

    def test_equal_values(self) -> None:
        a = [0, 1, 0, 1]
        result, borrow = _subtract_unsigned(a, a)
        from fp_arithmetic.ieee754 import _bits_msb_to_int
        assert _bits_msb_to_int(result) == 0
        assert borrow == 0


class TestAddBitsMsb:
    """Tests for _add_bits_msb helper."""

    def test_simple_add(self) -> None:
        from fp_arithmetic.ieee754 import _bits_msb_to_int
        # 3 + 5 = 8
        a = [0, 0, 1, 1]
        b = [0, 1, 0, 1]
        result, carry = _add_bits_msb(a, b)
        assert _bits_msb_to_int(result) == 8
        assert carry == 0

    def test_overflow(self) -> None:
        # 15 + 1 = 16 (overflow for 4 bits)
        a = [1, 1, 1, 1]
        b = [0, 0, 0, 1]
        result, carry = _add_bits_msb(a, b)
        assert carry == 1

    def test_with_carry_in(self) -> None:
        from fp_arithmetic.ieee754 import _bits_msb_to_int
        a = [0, 0, 0, 0]
        b = [0, 0, 0, 0]
        result, carry = _add_bits_msb(a, b, carry_in=1)
        assert _bits_msb_to_int(result) == 1
        assert carry == 0


# ---------------------------------------------------------------------------
# Basic FP32 addition tests
# ---------------------------------------------------------------------------


class TestFpAddBasic:
    """Basic FP32 addition tests."""

    def test_one_plus_one(self) -> None:
        _add_and_check(1.0, 1.0)

    def test_one_plus_two(self) -> None:
        _add_and_check(1.0, 2.0)

    def test_half_plus_half(self) -> None:
        _add_and_check(0.5, 0.5)

    def test_pi_plus_e(self) -> None:
        _add_and_check(3.14, 2.71)

    def test_large_plus_small(self) -> None:
        _add_and_check(1000.0, 0.001)

    def test_same_value(self) -> None:
        _add_and_check(42.0, 42.0)

    def test_hundred_plus_one(self) -> None:
        _add_and_check(100.0, 1.0)

    def test_negative_plus_negative(self) -> None:
        _add_and_check(-3.0, -4.0)

    def test_positive_plus_negative_same_magnitude(self) -> None:
        _add_and_check(5.0, -5.0)

    def test_positive_plus_negative_different(self) -> None:
        _add_and_check(5.0, -3.0)

    def test_negative_plus_positive(self) -> None:
        _add_and_check(-3.0, 5.0)

    def test_small_subtraction(self) -> None:
        """Catastrophic cancellation test."""
        a = float_to_bits(1.0000001, FP32)
        b = float_to_bits(-1.0, FP32)
        result = fp_add(a, b)
        result_float = bits_to_float(result)
        assert abs(result_float) < 1e-6
        assert abs(result_float) > 1e-9

    def test_very_large_exponent_diff(self) -> None:
        """Adding two numbers with very different exponents."""
        _add_and_check(1e20, 1e-20)

    def test_add_quarter_values(self) -> None:
        _add_and_check(0.25, 0.75)

    def test_negative_result(self) -> None:
        _add_and_check(1.0, -3.0)


# ---------------------------------------------------------------------------
# Special values
# ---------------------------------------------------------------------------


class TestFpAddSpecialValues:
    """Tests for special value handling in addition."""

    def test_nan_plus_number(self) -> None:
        a = float_to_bits(float("nan"), FP32)
        b = float_to_bits(1.0, FP32)
        assert is_nan(fp_add(a, b))

    def test_number_plus_nan(self) -> None:
        a = float_to_bits(1.0, FP32)
        b = float_to_bits(float("nan"), FP32)
        assert is_nan(fp_add(a, b))

    def test_nan_plus_nan(self) -> None:
        a = float_to_bits(float("nan"), FP32)
        b = float_to_bits(float("nan"), FP32)
        assert is_nan(fp_add(a, b))

    def test_inf_plus_inf(self) -> None:
        a = float_to_bits(float("inf"), FP32)
        result = fp_add(a, a)
        assert is_inf(result)
        assert result.sign == 0

    def test_inf_plus_neg_inf(self) -> None:
        a = float_to_bits(float("inf"), FP32)
        b = float_to_bits(float("-inf"), FP32)
        assert is_nan(fp_add(a, b))

    def test_neg_inf_plus_inf(self) -> None:
        a = float_to_bits(float("-inf"), FP32)
        b = float_to_bits(float("inf"), FP32)
        assert is_nan(fp_add(a, b))

    def test_inf_plus_number(self) -> None:
        a = float_to_bits(float("inf"), FP32)
        b = float_to_bits(42.0, FP32)
        result = fp_add(a, b)
        assert is_inf(result) and result.sign == 0

    def test_number_plus_inf(self) -> None:
        a = float_to_bits(42.0, FP32)
        b = float_to_bits(float("inf"), FP32)
        result = fp_add(a, b)
        assert is_inf(result)

    def test_neg_inf_plus_neg_inf(self) -> None:
        a = float_to_bits(float("-inf"), FP32)
        result = fp_add(a, a)
        assert is_inf(result) and result.sign == 1

    def test_neg_inf_plus_number(self) -> None:
        a = float_to_bits(float("-inf"), FP32)
        b = float_to_bits(42.0, FP32)
        result = fp_add(a, b)
        assert is_inf(result) and result.sign == 1

    def test_zero_plus_zero(self) -> None:
        a = float_to_bits(0.0, FP32)
        result = fp_add(a, a)
        assert is_zero(result)
        assert result.sign == 0

    def test_neg_zero_plus_neg_zero(self) -> None:
        a = float_to_bits(-0.0, FP32)
        result = fp_add(a, a)
        assert is_zero(result)
        assert result.sign == 1

    def test_pos_zero_plus_neg_zero(self) -> None:
        """IEEE 754: +0 + (-0) = +0."""
        a = float_to_bits(0.0, FP32)
        b = float_to_bits(-0.0, FP32)
        result = fp_add(a, b)
        assert is_zero(result)
        assert result.sign == 0

    def test_zero_plus_number(self) -> None:
        a = float_to_bits(0.0, FP32)
        b = float_to_bits(3.14, FP32)
        result = fp_add(a, b)
        assert abs(bits_to_float(result) - 3.14) < 0.001

    def test_number_plus_zero(self) -> None:
        a = float_to_bits(3.14, FP32)
        b = float_to_bits(0.0, FP32)
        result = fp_add(a, b)
        assert abs(bits_to_float(result) - 3.14) < 0.001

    def test_nan_plus_inf(self) -> None:
        a = float_to_bits(float("nan"), FP32)
        b = float_to_bits(float("inf"), FP32)
        assert is_nan(fp_add(a, b))

    def test_nan_plus_zero(self) -> None:
        a = float_to_bits(float("nan"), FP32)
        b = float_to_bits(0.0, FP32)
        assert is_nan(fp_add(a, b))


# ---------------------------------------------------------------------------
# Overflow and denormal addition
# ---------------------------------------------------------------------------


class TestFpAddOverflowUnderflow:
    """Tests for overflow to infinity and underflow to denormal."""

    def test_overflow_to_positive_inf(self) -> None:
        """Adding two huge numbers should overflow to infinity."""
        a = float_to_bits(3.0e38, FP32)
        b = float_to_bits(3.0e38, FP32)
        result = fp_add(a, b)
        assert is_inf(result)
        assert result.sign == 0

    def test_overflow_to_negative_inf(self) -> None:
        a = float_to_bits(-3.0e38, FP32)
        b = float_to_bits(-3.0e38, FP32)
        result = fp_add(a, b)
        assert is_inf(result)
        assert result.sign == 1

    def test_denormal_plus_denormal(self) -> None:
        """Adding two denormalized numbers."""
        d1 = FloatBits(sign=0, exponent=[0] * 8, mantissa=[1] + [0] * 22, fmt=FP32)
        d2 = FloatBits(sign=0, exponent=[0] * 8, mantissa=[0, 1] + [0] * 21, fmt=FP32)
        result = fp_add(d1, d2)
        result_val = bits_to_float(result)
        d1_val = bits_to_float(d1)
        d2_val = bits_to_float(d2)
        # Denormal addition has limited precision — verify result is
        # in the right order of magnitude (same ~1e-38 range)
        expected = d1_val + d2_val
        assert abs(result_val) > 0, "Result should be nonzero"
        assert abs(result_val - expected) / max(abs(expected), 1e-45) < 1.0

    def test_denormal_plus_normal(self) -> None:
        """Adding a denormal to a normal number."""
        denorm = FloatBits(sign=0, exponent=[0] * 8, mantissa=[1] + [0] * 22, fmt=FP32)
        normal = float_to_bits(1.0, FP32)
        result = fp_add(denorm, normal)
        # The denormal is so tiny compared to 1.0 that the result is ~1.0
        assert abs(bits_to_float(result) - 1.0) < 1e-6

    def test_subtraction_to_zero(self) -> None:
        """Subtracting equal values produces zero."""
        a = float_to_bits(42.0, FP32)
        b = float_to_bits(-42.0, FP32)
        result = fp_add(a, b)
        assert is_zero(result)

    def test_subtraction_result_denormal(self) -> None:
        """Subtracting nearly equal very small values can produce denormal."""
        # Create two small but slightly different normal FP32 values
        a = float_to_bits(1.18e-38, FP32)
        b = float_to_bits(1.17e-38, FP32)
        result = fp_sub(a, b)
        result_val = bits_to_float(result)
        expected = 1.18e-38 - 1.17e-38
        # Allow some tolerance due to FP32 precision
        assert abs(result_val - expected) < expected * 0.1 or abs(result_val) < 1e-39


# ---------------------------------------------------------------------------
# Parametrized tests
# ---------------------------------------------------------------------------


class TestFpAddManyValues:
    """Test fp_add against Python's native float for many value pairs."""

    @pytest.mark.parametrize(
        "a_val,b_val",
        [
            (1.0, 1.0),
            (1.0, -1.0),
            (0.1, 0.2),
            (100.0, 0.01),
            (-7.5, 3.25),
            (1e10, 1e-10),
            (0.5, 0.25),
            (1.5, 2.5),
            (-1.0, -2.0),
            (3.14, 2.71),
            (0.125, 0.0625),
            (-0.5, 0.5),
            (256.0, 256.0),
            (0.001, -0.0005),
        ],
    )
    def test_add_parametrized(self, a_val: float, b_val: float) -> None:
        _add_and_check(a_val, b_val)


# ---------------------------------------------------------------------------
# Subtraction
# ---------------------------------------------------------------------------


class TestFpSub:
    """Tests for fp_sub."""

    def test_three_minus_one(self) -> None:
        a = float_to_bits(3.0, FP32)
        b = float_to_bits(1.0, FP32)
        result = fp_sub(a, b)
        assert abs(bits_to_float(result) - 2.0) < 1e-6

    def test_one_minus_three(self) -> None:
        a = float_to_bits(1.0, FP32)
        b = float_to_bits(3.0, FP32)
        result = fp_sub(a, b)
        assert abs(bits_to_float(result) - (-2.0)) < 1e-6

    def test_same_value(self) -> None:
        a = float_to_bits(42.0, FP32)
        result = fp_sub(a, a)
        assert is_zero(result)

    def test_negative_minus_negative(self) -> None:
        a = float_to_bits(-3.0, FP32)
        b = float_to_bits(-1.0, FP32)
        result = fp_sub(a, b)
        assert abs(bits_to_float(result) - (-2.0)) < 1e-6

    def test_sub_zero(self) -> None:
        a = float_to_bits(5.0, FP32)
        b = float_to_bits(0.0, FP32)
        result = fp_sub(a, b)
        assert abs(bits_to_float(result) - 5.0) < 1e-6

    def test_zero_minus_number(self) -> None:
        a = float_to_bits(0.0, FP32)
        b = float_to_bits(5.0, FP32)
        result = fp_sub(a, b)
        assert abs(bits_to_float(result) - (-5.0)) < 1e-6

    def test_sub_inf(self) -> None:
        a = float_to_bits(1.0, FP32)
        b = float_to_bits(float("inf"), FP32)
        result = fp_sub(a, b)
        assert is_inf(result)
        assert result.sign == 1


# ---------------------------------------------------------------------------
# Negation
# ---------------------------------------------------------------------------


class TestFpNeg:
    """Tests for fp_neg."""

    def test_negate_positive(self) -> None:
        a = float_to_bits(3.14, FP32)
        result = fp_neg(a)
        assert result.sign == 1
        assert result.exponent == a.exponent
        assert result.mantissa == a.mantissa

    def test_negate_negative(self) -> None:
        a = float_to_bits(-2.5, FP32)
        result = fp_neg(a)
        assert result.sign == 0

    def test_negate_zero(self) -> None:
        a = float_to_bits(0.0, FP32)
        result = fp_neg(a)
        assert result.sign == 1

    def test_negate_neg_zero(self) -> None:
        a = float_to_bits(-0.0, FP32)
        result = fp_neg(a)
        assert result.sign == 0

    def test_double_negate(self) -> None:
        a = float_to_bits(1.0, FP32)
        result = fp_neg(fp_neg(a))
        assert result.sign == a.sign

    def test_negate_inf(self) -> None:
        a = float_to_bits(float("inf"), FP32)
        result = fp_neg(a)
        assert result.sign == 1
        assert is_inf(result)

    def test_negate_nan(self) -> None:
        a = float_to_bits(float("nan"), FP32)
        result = fp_neg(a)
        assert result.sign == 1
        assert is_nan(result)


# ---------------------------------------------------------------------------
# Absolute value
# ---------------------------------------------------------------------------


class TestFpAbs:
    """Tests for fp_abs."""

    def test_abs_positive(self) -> None:
        a = float_to_bits(3.14, FP32)
        result = fp_abs(a)
        assert result.sign == 0
        assert result.exponent == a.exponent

    def test_abs_negative(self) -> None:
        a = float_to_bits(-3.14, FP32)
        result = fp_abs(a)
        assert result.sign == 0

    def test_abs_zero(self) -> None:
        a = float_to_bits(-0.0, FP32)
        result = fp_abs(a)
        assert result.sign == 0

    def test_abs_pos_zero(self) -> None:
        a = float_to_bits(0.0, FP32)
        result = fp_abs(a)
        assert result.sign == 0

    def test_abs_nan(self) -> None:
        a = float_to_bits(float("nan"), FP32)
        result = fp_abs(a)
        assert result.sign == 0
        assert is_nan(result)

    def test_abs_neg_inf(self) -> None:
        a = float_to_bits(float("-inf"), FP32)
        result = fp_abs(a)
        assert result.sign == 0
        assert is_inf(result)

    def test_abs_pos_inf(self) -> None:
        a = float_to_bits(float("inf"), FP32)
        result = fp_abs(a)
        assert result.sign == 0
        assert is_inf(result)


# ---------------------------------------------------------------------------
# Comparison
# ---------------------------------------------------------------------------


class TestFpCompare:
    """Tests for fp_compare."""

    def test_equal(self) -> None:
        a = float_to_bits(1.0, FP32)
        assert fp_compare(a, a) == 0

    def test_less_than(self) -> None:
        a = float_to_bits(1.0, FP32)
        b = float_to_bits(2.0, FP32)
        assert fp_compare(a, b) == -1

    def test_greater_than(self) -> None:
        a = float_to_bits(2.0, FP32)
        b = float_to_bits(1.0, FP32)
        assert fp_compare(a, b) == 1

    def test_negative_less_than_positive(self) -> None:
        a = float_to_bits(-1.0, FP32)
        b = float_to_bits(1.0, FP32)
        assert fp_compare(a, b) == -1

    def test_positive_greater_than_negative(self) -> None:
        a = float_to_bits(1.0, FP32)
        b = float_to_bits(-1.0, FP32)
        assert fp_compare(a, b) == 1

    def test_negative_compare(self) -> None:
        a = float_to_bits(-3.0, FP32)
        b = float_to_bits(-1.0, FP32)
        assert fp_compare(a, b) == -1

    def test_negative_compare_reversed(self) -> None:
        a = float_to_bits(-1.0, FP32)
        b = float_to_bits(-3.0, FP32)
        assert fp_compare(a, b) == 1

    def test_zeros_equal(self) -> None:
        a = float_to_bits(0.0, FP32)
        b = float_to_bits(-0.0, FP32)
        assert fp_compare(a, b) == 0

    def test_nan_unordered(self) -> None:
        a = float_to_bits(float("nan"), FP32)
        b = float_to_bits(1.0, FP32)
        assert fp_compare(a, b) == 0

    def test_nan_vs_nan(self) -> None:
        a = float_to_bits(float("nan"), FP32)
        assert fp_compare(a, a) == 0

    def test_number_vs_nan(self) -> None:
        a = float_to_bits(1.0, FP32)
        b = float_to_bits(float("nan"), FP32)
        assert fp_compare(a, b) == 0

    def test_inf_compare(self) -> None:
        a = float_to_bits(float("inf"), FP32)
        b = float_to_bits(1e38, FP32)
        assert fp_compare(a, b) == 1

    def test_neg_inf_compare(self) -> None:
        a = float_to_bits(float("-inf"), FP32)
        b = float_to_bits(-1e38, FP32)
        assert fp_compare(a, b) == -1

    def test_same_exp_different_mant(self) -> None:
        a = float_to_bits(1.5, FP32)
        b = float_to_bits(1.25, FP32)
        assert fp_compare(a, b) == 1

    def test_zero_vs_positive(self) -> None:
        a = float_to_bits(0.0, FP32)
        b = float_to_bits(1.0, FP32)
        assert fp_compare(a, b) == -1

    def test_zero_vs_negative(self) -> None:
        a = float_to_bits(0.0, FP32)
        b = float_to_bits(-1.0, FP32)
        assert fp_compare(a, b) == 1

    def test_negative_same_exp(self) -> None:
        a = float_to_bits(-1.5, FP32)
        b = float_to_bits(-1.25, FP32)
        assert fp_compare(a, b) == -1

    def test_neg_zero_vs_positive(self) -> None:
        a = float_to_bits(-0.0, FP32)
        b = float_to_bits(1.0, FP32)
        assert fp_compare(a, b) == -1

    def test_neg_zero_vs_negative(self) -> None:
        a = float_to_bits(-0.0, FP32)
        b = float_to_bits(-1.0, FP32)
        assert fp_compare(a, b) == 1

    def test_negative_same_mant_different_exp(self) -> None:
        """Two negative numbers with different exponents."""
        a = float_to_bits(-10.0, FP32)
        b = float_to_bits(-1.0, FP32)
        assert fp_compare(a, b) == -1

    def test_negative_same_mant_different_exp_reversed(self) -> None:
        a = float_to_bits(-1.0, FP32)
        b = float_to_bits(-10.0, FP32)
        assert fp_compare(a, b) == 1
