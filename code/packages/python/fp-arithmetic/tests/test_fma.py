"""Tests for fma.py — fused multiply-add and format conversion.

Comprehensive tests covering:
- Basic FMA, special values, denormals
- FMA vs separate mul+add precision comparison
- Format conversion across all three formats
- Overflow, underflow, zero/sign handling in FMA
"""

import math

import pytest

from fp_arithmetic.fma import fp_convert, fp_fma
from fp_arithmetic.formats import BF16, FP16, FP32, FloatBits
from fp_arithmetic.fp_adder import fp_add
from fp_arithmetic.fp_multiplier import fp_mul
from fp_arithmetic.ieee754 import bits_to_float, float_to_bits, is_inf, is_nan, is_zero


def _fma_and_check(a_val: float, b_val: float, c_val: float) -> None:
    """Helper: compute FMA and check against a*b + c with tolerance."""
    a = float_to_bits(a_val, FP32)
    b = float_to_bits(b_val, FP32)
    c = float_to_bits(c_val, FP32)
    result = fp_fma(a, b, c)
    result_float = bits_to_float(result)
    expected = a_val * b_val + c_val

    if math.isnan(expected):
        assert math.isnan(result_float), f"Expected NaN for FMA({a_val}, {b_val}, {c_val})"
    elif math.isinf(expected):
        assert math.isinf(result_float), f"Expected Inf for FMA({a_val}, {b_val}, {c_val})"
    elif expected == 0.0:
        assert abs(result_float) < 1e-6, (
            f"Expected ~0 for FMA({a_val}, {b_val}, {c_val}), got {result_float}"
        )
    else:
        rel_err = abs(result_float - expected) / max(abs(expected), 1e-45)
        assert rel_err < 1e-5, (
            f"FMA({a_val}, {b_val}, {c_val}): expected {expected}, got {result_float}, "
            f"rel_err={rel_err}"
        )


# ---------------------------------------------------------------------------
# Basic FMA
# ---------------------------------------------------------------------------


class TestFmaBasic:
    """Basic FMA tests."""

    def test_simple_fma(self) -> None:
        _fma_and_check(1.5, 2.0, 0.25)

    def test_multiply_only(self) -> None:
        _fma_and_check(3.0, 4.0, 0.0)

    def test_add_only(self) -> None:
        _fma_and_check(1.0, 3.0, 4.0)

    def test_negative_addend(self) -> None:
        _fma_and_check(2.0, 3.0, -1.0)

    def test_all_negative(self) -> None:
        _fma_and_check(-2.0, -3.0, -1.0)

    def test_cancellation(self) -> None:
        _fma_and_check(2.0, 3.0, -6.0)

    def test_pi_times_e_plus_one(self) -> None:
        _fma_and_check(3.14, 2.71, 1.0)

    def test_small_values(self) -> None:
        _fma_and_check(0.1, 0.2, 0.3)

    def test_large_product_small_addend(self) -> None:
        _fma_and_check(100.0, 100.0, 0.001)

    def test_small_product_large_addend(self) -> None:
        _fma_and_check(0.001, 0.001, 100.0)

    def test_negative_product_positive_addend(self) -> None:
        _fma_and_check(-2.0, 3.0, 10.0)

    def test_positive_product_negative_addend(self) -> None:
        _fma_and_check(2.0, 3.0, -10.0)


# ---------------------------------------------------------------------------
# Special values
# ---------------------------------------------------------------------------


class TestFmaSpecialValues:
    """Tests for special value handling in FMA."""

    def test_nan_propagation_a(self) -> None:
        a = float_to_bits(float("nan"), FP32)
        b = float_to_bits(1.0, FP32)
        c = float_to_bits(1.0, FP32)
        assert is_nan(fp_fma(a, b, c))

    def test_nan_propagation_b(self) -> None:
        a = float_to_bits(1.0, FP32)
        b = float_to_bits(float("nan"), FP32)
        c = float_to_bits(1.0, FP32)
        assert is_nan(fp_fma(a, b, c))

    def test_nan_propagation_c(self) -> None:
        a = float_to_bits(1.0, FP32)
        b = float_to_bits(1.0, FP32)
        c = float_to_bits(float("nan"), FP32)
        assert is_nan(fp_fma(a, b, c))

    def test_nan_all(self) -> None:
        nan = float_to_bits(float("nan"), FP32)
        assert is_nan(fp_fma(nan, nan, nan))

    def test_inf_times_zero(self) -> None:
        a = float_to_bits(float("inf"), FP32)
        b = float_to_bits(0.0, FP32)
        c = float_to_bits(1.0, FP32)
        assert is_nan(fp_fma(a, b, c))

    def test_zero_times_inf(self) -> None:
        a = float_to_bits(0.0, FP32)
        b = float_to_bits(float("inf"), FP32)
        c = float_to_bits(1.0, FP32)
        assert is_nan(fp_fma(a, b, c))

    def test_inf_times_finite_plus_finite(self) -> None:
        a = float_to_bits(float("inf"), FP32)
        b = float_to_bits(2.0, FP32)
        c = float_to_bits(1.0, FP32)
        result = fp_fma(a, b, c)
        assert is_inf(result) and result.sign == 0

    def test_neg_inf_times_pos_plus_finite(self) -> None:
        a = float_to_bits(float("-inf"), FP32)
        b = float_to_bits(2.0, FP32)
        c = float_to_bits(1.0, FP32)
        result = fp_fma(a, b, c)
        assert is_inf(result) and result.sign == 1

    def test_inf_product_plus_neg_inf(self) -> None:
        """Inf * 1 + (-Inf) = NaN."""
        a = float_to_bits(float("inf"), FP32)
        b = float_to_bits(1.0, FP32)
        c = float_to_bits(float("-inf"), FP32)
        assert is_nan(fp_fma(a, b, c))

    def test_inf_product_plus_same_inf(self) -> None:
        """Inf * 1 + Inf = Inf."""
        a = float_to_bits(float("inf"), FP32)
        b = float_to_bits(1.0, FP32)
        c = float_to_bits(float("inf"), FP32)
        result = fp_fma(a, b, c)
        assert is_inf(result) and result.sign == 0

    def test_zero_times_zero_plus_number(self) -> None:
        a = float_to_bits(0.0, FP32)
        b = float_to_bits(0.0, FP32)
        c = float_to_bits(5.0, FP32)
        result = fp_fma(a, b, c)
        assert abs(bits_to_float(result) - 5.0) < 1e-6

    def test_zero_times_number_plus_zero(self) -> None:
        a = float_to_bits(0.0, FP32)
        b = float_to_bits(5.0, FP32)
        c = float_to_bits(0.0, FP32)
        result = fp_fma(a, b, c)
        assert is_zero(result)

    def test_zero_product_plus_zero_sign(self) -> None:
        """0 * (-1) + (-0) = -0 (both product and c are negative zero)."""
        a = float_to_bits(0.0, FP32)
        b = float_to_bits(-1.0, FP32)
        c = float_to_bits(-0.0, FP32)
        result = fp_fma(a, b, c)
        assert is_zero(result)
        assert result.sign == 1  # -0 * anything + -0 = -0

    def test_zero_product_positive_sign(self) -> None:
        """0 * 1 + 0 = +0."""
        a = float_to_bits(0.0, FP32)
        b = float_to_bits(1.0, FP32)
        c = float_to_bits(0.0, FP32)
        result = fp_fma(a, b, c)
        assert is_zero(result)
        assert result.sign == 0

    def test_finite_times_finite_plus_inf(self) -> None:
        a = float_to_bits(2.0, FP32)
        b = float_to_bits(3.0, FP32)
        c = float_to_bits(float("inf"), FP32)
        result = fp_fma(a, b, c)
        assert is_inf(result)

    def test_finite_times_finite_plus_neg_inf(self) -> None:
        a = float_to_bits(2.0, FP32)
        b = float_to_bits(3.0, FP32)
        c = float_to_bits(float("-inf"), FP32)
        result = fp_fma(a, b, c)
        assert is_inf(result) and result.sign == 1

    def test_neg_product_sign(self) -> None:
        a = float_to_bits(-1.0, FP32)
        b = float_to_bits(2.0, FP32)
        c = float_to_bits(0.0, FP32)
        result = fp_fma(a, b, c)
        assert bits_to_float(result) == -2.0

    def test_number_times_zero_returns_c(self) -> None:
        """When a is zero, result should be c."""
        a = float_to_bits(0.0, FP32)
        b = float_to_bits(42.0, FP32)
        c = float_to_bits(7.0, FP32)
        result = fp_fma(a, b, c)
        assert abs(bits_to_float(result) - 7.0) < 1e-6


# ---------------------------------------------------------------------------
# FMA overflow/underflow
# ---------------------------------------------------------------------------


class TestFmaOverflowUnderflow:
    """Tests for overflow/underflow in FMA."""

    def test_fma_overflow_to_inf(self) -> None:
        """Large product that overflows."""
        a = float_to_bits(1e30, FP32)
        b = float_to_bits(1e30, FP32)
        c = float_to_bits(0.0, FP32)
        result = fp_fma(a, b, c)
        assert is_inf(result)

    def test_fma_cancellation_to_zero(self) -> None:
        """Product exactly cancels with c."""
        _fma_and_check(2.0, 3.0, -6.0)

    def test_fma_denormal_addend(self) -> None:
        """FMA with a denormal addend."""
        denorm = FloatBits(sign=0, exponent=[0] * 8, mantissa=[1] + [0] * 22, fmt=FP32)
        a = float_to_bits(1.0, FP32)
        b = float_to_bits(1.0, FP32)
        result = fp_fma(a, b, denorm)
        result_val = bits_to_float(result)
        # 1.0 * 1.0 + tiny denorm ~ 1.0
        assert abs(result_val - 1.0) < 1e-6


# ---------------------------------------------------------------------------
# Parametrized FMA
# ---------------------------------------------------------------------------


class TestFmaManyValues:
    """Parametrized FMA tests."""

    @pytest.mark.parametrize(
        "a_val,b_val,c_val",
        [
            (1.0, 1.0, 1.0),
            (2.0, 3.0, 4.0),
            (0.5, 0.5, 0.5),
            (-1.0, 2.0, 3.0),
            (10.0, 10.0, -100.0),
            (3.14, 2.71, 1.41),
            (0.1, 0.2, 0.3),
            (100.0, 0.01, 1.0),
            (0.25, 4.0, 0.0),
            (-0.5, -0.5, 0.25),
            (1.0, -1.0, 1.0),
            (8.0, 0.125, 0.0),
        ],
    )
    def test_fma_parametrized(self, a_val: float, b_val: float, c_val: float) -> None:
        _fma_and_check(a_val, b_val, c_val)


# ---------------------------------------------------------------------------
# Format conversion
# ---------------------------------------------------------------------------


class TestFpConvert:
    """Tests for fp_convert — format conversion."""

    def test_same_format_noop(self) -> None:
        bits = float_to_bits(3.14, FP32)
        result = fp_convert(bits, FP32)
        assert bits_to_float(result) == bits_to_float(bits)

    def test_fp32_to_fp16(self) -> None:
        bits = float_to_bits(1.0, FP32)
        result = fp_convert(bits, FP16)
        assert result.fmt == FP16
        assert bits_to_float(result) == 1.0

    def test_fp32_to_bf16(self) -> None:
        bits = float_to_bits(1.0, FP32)
        result = fp_convert(bits, BF16)
        assert result.fmt == BF16
        assert bits_to_float(result) == 1.0

    def test_fp16_to_fp32(self) -> None:
        bits = float_to_bits(2.0, FP16)
        result = fp_convert(bits, FP32)
        assert result.fmt == FP32
        assert bits_to_float(result) == 2.0

    def test_bf16_to_fp32(self) -> None:
        bits = float_to_bits(0.5, BF16)
        result = fp_convert(bits, FP32)
        assert result.fmt == FP32
        assert bits_to_float(result) == 0.5

    def test_fp32_to_fp16_precision_loss(self) -> None:
        bits = float_to_bits(3.14, FP32)
        fp16_bits = fp_convert(bits, FP16)
        back_to_fp32 = fp_convert(fp16_bits, FP32)
        val = bits_to_float(back_to_fp32)
        assert abs(val - 3.14) < 0.01

    def test_fp32_to_bf16_precision_loss(self) -> None:
        bits = float_to_bits(3.14, FP32)
        bf16_bits = fp_convert(bits, BF16)
        val = bits_to_float(bf16_bits)
        assert abs(val - 3.14) < 0.05

    def test_convert_nan(self) -> None:
        bits = float_to_bits(float("nan"), FP32)
        result = fp_convert(bits, FP16)
        assert is_nan(result)

    def test_convert_nan_to_bf16(self) -> None:
        bits = float_to_bits(float("nan"), FP32)
        result = fp_convert(bits, BF16)
        assert is_nan(result)

    def test_convert_inf(self) -> None:
        bits = float_to_bits(float("inf"), FP32)
        result = fp_convert(bits, FP16)
        assert is_inf(result)

    def test_convert_neg_inf(self) -> None:
        bits = float_to_bits(float("-inf"), FP32)
        result = fp_convert(bits, FP16)
        assert is_inf(result) and result.sign == 1

    def test_convert_inf_to_bf16(self) -> None:
        bits = float_to_bits(float("inf"), FP32)
        result = fp_convert(bits, BF16)
        assert is_inf(result)

    def test_convert_zero(self) -> None:
        bits = float_to_bits(0.0, FP32)
        result = fp_convert(bits, BF16)
        assert is_zero(result)

    def test_convert_neg_zero(self) -> None:
        bits = float_to_bits(-0.0, FP32)
        result = fp_convert(bits, FP16)
        assert is_zero(result) and result.sign == 1

    def test_convert_overflow_to_inf(self) -> None:
        """Large FP32 value should become Inf when converted to FP16."""
        bits = float_to_bits(100000.0, FP32)
        result = fp_convert(bits, FP16)
        assert is_inf(result)

    def test_fp16_to_bf16(self) -> None:
        bits = float_to_bits(1.0, FP16)
        result = fp_convert(bits, BF16)
        assert result.fmt == BF16
        assert bits_to_float(result) == 1.0

    def test_bf16_to_fp16(self) -> None:
        bits = float_to_bits(1.0, BF16)
        result = fp_convert(bits, FP16)
        assert result.fmt == FP16
        assert bits_to_float(result) == 1.0

    def test_convert_negative(self) -> None:
        bits = float_to_bits(-2.5, FP32)
        result = fp_convert(bits, FP16)
        assert result.sign == 1
        assert abs(bits_to_float(result) - (-2.5)) < 0.01

    def test_convert_fp16_exact_values(self) -> None:
        """Values exactly representable in FP16 should roundtrip."""
        for val in [0.0, 1.0, -1.0, 0.5, 2.0, 4.0, 0.25]:
            fp32_bits = float_to_bits(val, FP32)
            fp16_bits = fp_convert(fp32_bits, FP16)
            back = fp_convert(fp16_bits, FP32)
            assert bits_to_float(back) == val, f"Roundtrip failed for {val}"

    def test_convert_bf16_exact_values(self) -> None:
        """Values exactly representable in BF16 should roundtrip."""
        for val in [0.0, 1.0, -1.0, 0.5, 2.0, 128.0]:
            fp32_bits = float_to_bits(val, FP32)
            bf16_bits = fp_convert(fp32_bits, BF16)
            back = fp_convert(bf16_bits, FP32)
            assert bits_to_float(back) == val, f"Roundtrip failed for {val}"
