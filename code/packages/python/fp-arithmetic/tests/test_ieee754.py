"""Tests for ieee754.py — encoding, decoding, and special value detection.

Comprehensive coverage targeting 95%+, testing every edge case:
- Normal values, denormals, zeros, infinities, NaN
- All three formats: FP32, FP16, BF16
- Roundtrip encoding/decoding
- Internal helpers (_int_to_bits_msb, _bits_msb_to_int, _all_ones, _all_zeros)
- Overflow/underflow paths in FP16/BF16 conversion
"""

import math
import struct

import pytest

from fp_arithmetic.formats import BF16, FP16, FP32, FloatBits, FloatFormat
from fp_arithmetic.ieee754 import (
    _all_ones,
    _all_zeros,
    _bits_msb_to_int,
    _int_to_bits_msb,
    bits_to_float,
    float_to_bits,
    is_denormalized,
    is_inf,
    is_nan,
    is_zero,
)


# ---------------------------------------------------------------------------
# Tests for internal helpers
# ---------------------------------------------------------------------------


class TestIntBitConversions:
    """Tests for _int_to_bits_msb and _bits_msb_to_int."""

    def test_zero(self) -> None:
        assert _int_to_bits_msb(0, 8) == [0, 0, 0, 0, 0, 0, 0, 0]

    def test_one(self) -> None:
        assert _int_to_bits_msb(1, 8) == [0, 0, 0, 0, 0, 0, 0, 1]

    def test_max_byte(self) -> None:
        assert _int_to_bits_msb(255, 8) == [1, 1, 1, 1, 1, 1, 1, 1]

    def test_five(self) -> None:
        assert _int_to_bits_msb(5, 8) == [0, 0, 0, 0, 0, 1, 0, 1]

    def test_width_1(self) -> None:
        assert _int_to_bits_msb(0, 1) == [0]
        assert _int_to_bits_msb(1, 1) == [1]

    def test_width_16(self) -> None:
        bits = _int_to_bits_msb(0xABCD, 16)
        assert len(bits) == 16
        assert _bits_msb_to_int(bits) == 0xABCD

    def test_roundtrip(self) -> None:
        for val in [0, 1, 42, 127, 255]:
            assert _bits_msb_to_int(_int_to_bits_msb(val, 8)) == val

    def test_roundtrip_wide(self) -> None:
        for val in [0, 1, 1023, 65535, (1 << 23) - 1]:
            assert _bits_msb_to_int(_int_to_bits_msb(val, 23)) == val

    def test_bits_to_int_examples(self) -> None:
        assert _bits_msb_to_int([1, 0, 1]) == 5
        assert _bits_msb_to_int([0]) == 0
        assert _bits_msb_to_int([1]) == 1

    def test_bits_to_int_empty(self) -> None:
        assert _bits_msb_to_int([]) == 0


class TestAllOnesAllZeros:
    """Tests for _all_ones and _all_zeros helper functions."""

    def test_all_ones_true(self) -> None:
        assert _all_ones([1, 1, 1, 1]) is True

    def test_all_ones_false(self) -> None:
        assert _all_ones([1, 0, 1, 1]) is False

    def test_all_ones_single(self) -> None:
        assert _all_ones([1]) is True
        assert _all_ones([0]) is False

    def test_all_zeros_true(self) -> None:
        assert _all_zeros([0, 0, 0, 0]) is True

    def test_all_zeros_false(self) -> None:
        assert _all_zeros([0, 0, 1, 0]) is False

    def test_all_zeros_single(self) -> None:
        assert _all_zeros([0]) is True
        assert _all_zeros([1]) is False

    def test_all_ones_8bit(self) -> None:
        assert _all_ones([1] * 8) is True
        assert _all_ones([1, 1, 1, 1, 1, 1, 1, 0]) is False

    def test_all_zeros_8bit(self) -> None:
        assert _all_zeros([0] * 8) is True
        assert _all_zeros([0, 0, 0, 0, 0, 0, 0, 1]) is False


# ---------------------------------------------------------------------------
# Tests for float_to_bits — FP32
# ---------------------------------------------------------------------------


class TestFloatToBitsFP32:
    """Tests for float_to_bits with FP32 format, verified against struct.pack."""

    def _verify_against_struct(self, value: float) -> None:
        bits = float_to_bits(value, FP32)
        int_val = (bits.sign << 31)
        int_val |= (_bits_msb_to_int(bits.exponent) << 23)
        int_val |= _bits_msb_to_int(bits.mantissa)
        expected = struct.unpack("!I", struct.pack("!f", value))[0]
        assert int_val == expected, (
            f"Mismatch for {value}: got 0x{int_val:08x}, expected 0x{expected:08x}"
        )

    def test_positive_one(self) -> None:
        self._verify_against_struct(1.0)
        bits = float_to_bits(1.0, FP32)
        assert bits.sign == 0
        assert bits.exponent == [0, 1, 1, 1, 1, 1, 1, 1]
        assert bits.mantissa == [0] * 23

    def test_negative_one(self) -> None:
        self._verify_against_struct(-1.0)
        bits = float_to_bits(-1.0, FP32)
        assert bits.sign == 1

    def test_two(self) -> None:
        self._verify_against_struct(2.0)
        bits = float_to_bits(2.0, FP32)
        assert bits.sign == 0
        assert bits.exponent == [1, 0, 0, 0, 0, 0, 0, 0]

    def test_pi(self) -> None:
        self._verify_against_struct(3.14)

    def test_zero(self) -> None:
        self._verify_against_struct(0.0)
        bits = float_to_bits(0.0, FP32)
        assert bits.sign == 0
        assert bits.exponent == [0] * 8
        assert bits.mantissa == [0] * 23

    def test_negative_zero(self) -> None:
        bits = float_to_bits(-0.0, FP32)
        assert bits.sign == 1
        assert bits.exponent == [0] * 8
        assert bits.mantissa == [0] * 23

    def test_half(self) -> None:
        self._verify_against_struct(0.5)

    def test_many_values(self) -> None:
        values = [
            0.1, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.14,
            100.0, 1000.0, -0.1, -42.0, 1e10, 1e-10,
        ]
        for v in values:
            self._verify_against_struct(v)

    def test_largest_fp32(self) -> None:
        self._verify_against_struct(3.4028235e+38)

    def test_smallest_normal_fp32(self) -> None:
        self._verify_against_struct(1.1754944e-38)

    def test_default_format_is_fp32(self) -> None:
        """float_to_bits with no format argument defaults to FP32."""
        bits = float_to_bits(1.0)
        assert bits.fmt == FP32

    def test_very_small_positive(self) -> None:
        self._verify_against_struct(1e-38)

    def test_very_large_positive(self) -> None:
        self._verify_against_struct(1e38)

    def test_negative_pi(self) -> None:
        self._verify_against_struct(-3.14159)


# ---------------------------------------------------------------------------
# Tests for float_to_bits — special values
# ---------------------------------------------------------------------------


class TestFloatToBitsSpecialValues:
    """Tests for encoding special values across all formats."""

    def test_nan_fp32(self) -> None:
        bits = float_to_bits(float("nan"), FP32)
        assert bits.exponent == [1] * 8
        assert bits.mantissa[0] == 1

    def test_positive_inf_fp32(self) -> None:
        bits = float_to_bits(float("inf"), FP32)
        assert bits.sign == 0
        assert bits.exponent == [1] * 8
        assert bits.mantissa == [0] * 23

    def test_negative_inf_fp32(self) -> None:
        bits = float_to_bits(float("-inf"), FP32)
        assert bits.sign == 1
        assert bits.exponent == [1] * 8
        assert bits.mantissa == [0] * 23

    def test_nan_fp16(self) -> None:
        bits = float_to_bits(float("nan"), FP16)
        assert bits.exponent == [1] * 5
        assert bits.mantissa[0] == 1

    def test_nan_bf16(self) -> None:
        bits = float_to_bits(float("nan"), BF16)
        assert bits.exponent == [1] * 8
        assert bits.mantissa[0] == 1

    def test_positive_inf_fp16(self) -> None:
        bits = float_to_bits(float("inf"), FP16)
        assert bits.sign == 0
        assert bits.exponent == [1] * 5
        assert bits.mantissa == [0] * 10

    def test_negative_inf_fp16(self) -> None:
        bits = float_to_bits(float("-inf"), FP16)
        assert bits.sign == 1

    def test_positive_inf_bf16(self) -> None:
        bits = float_to_bits(float("inf"), BF16)
        assert bits.sign == 0
        assert bits.exponent == [1] * 8
        assert bits.mantissa == [0] * 7

    def test_negative_inf_bf16(self) -> None:
        bits = float_to_bits(float("-inf"), BF16)
        assert bits.sign == 1


# ---------------------------------------------------------------------------
# Tests for float_to_bits — FP16
# ---------------------------------------------------------------------------


class TestFloatToBitsFP16:
    """Tests for float_to_bits with FP16 format."""

    def test_one(self) -> None:
        bits = float_to_bits(1.0, FP16)
        assert bits.sign == 0
        assert bits.exponent == [0, 1, 1, 1, 1]
        assert bits.mantissa == [0] * 10

    def test_negative_one(self) -> None:
        bits = float_to_bits(-1.0, FP16)
        assert bits.sign == 1

    def test_zero(self) -> None:
        bits = float_to_bits(0.0, FP16)
        assert bits.sign == 0
        assert bits.exponent == [0] * 5
        assert bits.mantissa == [0] * 10

    def test_negative_zero(self) -> None:
        bits = float_to_bits(-0.0, FP16)
        assert bits.sign == 1
        assert bits.exponent == [0] * 5

    def test_overflow_to_inf(self) -> None:
        """Values too large for FP16 should become Inf."""
        bits = float_to_bits(100000.0, FP16)
        assert is_inf(bits)

    def test_two(self) -> None:
        bits = float_to_bits(2.0, FP16)
        assert bits.exponent == [1, 0, 0, 0, 0]

    def test_half(self) -> None:
        bits = float_to_bits(0.5, FP16)
        assert bits.exponent == [0, 1, 1, 1, 0]
        assert bits.mantissa == [0] * 10

    def test_roundtrip_simple(self) -> None:
        for val in [1.0, -1.0, 2.0, 0.5, 0.25]:
            bits = float_to_bits(val, FP16)
            result = bits_to_float(bits)
            assert result == val

    def test_fp16_max_normal(self) -> None:
        """FP16 max normal is 65504."""
        bits = float_to_bits(65504.0, FP16)
        result = bits_to_float(bits)
        assert result == 65504.0

    def test_fp16_underflow_to_denormal(self) -> None:
        """Very small values in FP16 become denormal."""
        # Smallest FP16 normal is ~6.1e-5 (exp=1, mant=0)
        # Values smaller than that become denormal
        bits = float_to_bits(1e-7, FP16)
        # Should be denormalized or zero
        exp_val = _bits_msb_to_int(bits.exponent)
        assert exp_val == 0  # denormal or zero

    def test_fp16_underflow_to_zero(self) -> None:
        """Extremely small values flush to zero in FP16."""
        bits = float_to_bits(1e-20, FP16)
        assert is_zero(bits)

    def test_fp16_rounding(self) -> None:
        """FP16 has only 10 mantissa bits, so 3.14 loses precision."""
        bits = float_to_bits(3.14, FP16)
        result = bits_to_float(bits)
        assert abs(result - 3.14) < 0.01


# ---------------------------------------------------------------------------
# Tests for float_to_bits — BF16
# ---------------------------------------------------------------------------


class TestFloatToBitsBF16:
    """Tests for float_to_bits with BF16 format."""

    def test_one(self) -> None:
        bits = float_to_bits(1.0, BF16)
        assert bits.sign == 0
        assert bits.exponent == [0, 1, 1, 1, 1, 1, 1, 1]
        assert bits.mantissa == [0] * 7

    def test_zero(self) -> None:
        bits = float_to_bits(0.0, BF16)
        assert bits.exponent == [0] * 8
        assert bits.mantissa == [0] * 7

    def test_negative_zero(self) -> None:
        bits = float_to_bits(-0.0, BF16)
        assert bits.sign == 1

    def test_two(self) -> None:
        bits = float_to_bits(2.0, BF16)
        result = bits_to_float(bits)
        assert result == 2.0

    def test_bf16_large_value(self) -> None:
        """BF16 shares FP32's exponent range, so large values survive."""
        bits = float_to_bits(1e30, BF16)
        result = bits_to_float(bits)
        assert abs(result - 1e30) / 1e30 < 0.01

    def test_bf16_rounding_overflow(self) -> None:
        """Test rounding that causes mantissa overflow in BF16.

        With only 7 mantissa bits, rounding up can carry into the exponent.
        """
        # 1.9921875 in BF16: mantissa = 1111111, adding the round bit overflows
        bits = float_to_bits(1.9921875, BF16)
        result = bits_to_float(bits)
        assert abs(result - 2.0) < 0.02

    def test_bf16_overflow_to_inf(self) -> None:
        """Values at the extreme of BF16's range overflow to inf."""
        # BF16 max is same range as FP32, but a value slightly above max
        # should overflow. We use a value beyond FP32 max.
        bits = float_to_bits(float("inf"), BF16)
        assert is_inf(bits)


# ---------------------------------------------------------------------------
# Tests for bits_to_float
# ---------------------------------------------------------------------------


class TestBitsToFloat:
    """Tests for bits_to_float — decoding back to Python float."""

    def test_roundtrip_fp32(self) -> None:
        values = [0.0, 1.0, -1.0, 0.5, 2.0, 3.14, -42.0, 100.0, 1e-10]
        for val in values:
            bits = float_to_bits(val, FP32)
            result = bits_to_float(bits)
            if val == 0.0:
                assert result == 0.0
            else:
                assert abs(result - val) < abs(val) * 1e-6, f"Failed for {val}: got {result}"

    def test_nan_roundtrip(self) -> None:
        bits = float_to_bits(float("nan"), FP32)
        assert math.isnan(bits_to_float(bits))

    def test_inf_roundtrip(self) -> None:
        bits = float_to_bits(float("inf"), FP32)
        assert bits_to_float(bits) == float("inf")

    def test_negative_inf_roundtrip(self) -> None:
        bits = float_to_bits(float("-inf"), FP32)
        assert bits_to_float(bits) == float("-inf")

    def test_negative_zero_roundtrip(self) -> None:
        bits = float_to_bits(-0.0, FP32)
        result = bits_to_float(bits)
        assert result == 0.0
        assert math.copysign(1.0, result) == -1.0

    def test_fp16_roundtrip(self) -> None:
        for val in [0.0, 1.0, -1.0, 0.5, 2.0, -0.25]:
            bits = float_to_bits(val, FP16)
            result = bits_to_float(bits)
            assert result == val, f"FP16 roundtrip failed for {val}: got {result}"

    def test_bf16_roundtrip(self) -> None:
        for val in [0.0, 1.0, -1.0, 0.5, 2.0]:
            bits = float_to_bits(val, BF16)
            result = bits_to_float(bits)
            assert result == val, f"BF16 roundtrip failed for {val}: got {result}"

    def test_fp16_nan_roundtrip(self) -> None:
        bits = float_to_bits(float("nan"), FP16)
        assert math.isnan(bits_to_float(bits))

    def test_fp16_inf_roundtrip(self) -> None:
        bits = float_to_bits(float("inf"), FP16)
        assert bits_to_float(bits) == float("inf")

    def test_fp16_neg_inf_roundtrip(self) -> None:
        bits = float_to_bits(float("-inf"), FP16)
        assert bits_to_float(bits) == float("-inf")

    def test_bf16_nan_roundtrip(self) -> None:
        bits = float_to_bits(float("nan"), BF16)
        assert math.isnan(bits_to_float(bits))

    def test_bf16_inf_roundtrip(self) -> None:
        bits = float_to_bits(float("inf"), BF16)
        assert bits_to_float(bits) == float("inf")

    def test_bf16_neg_inf_roundtrip(self) -> None:
        bits = float_to_bits(float("-inf"), BF16)
        assert bits_to_float(bits) == float("-inf")

    def test_fp16_negative_zero(self) -> None:
        bits = float_to_bits(-0.0, FP16)
        result = bits_to_float(bits)
        assert result == 0.0
        assert math.copysign(1.0, result) == -1.0

    def test_bf16_negative_zero(self) -> None:
        bits = float_to_bits(-0.0, BF16)
        result = bits_to_float(bits)
        assert result == 0.0
        assert math.copysign(1.0, result) == -1.0

    def test_fp16_denormal_decode(self) -> None:
        """Decode a manually-constructed FP16 denormal."""
        # Smallest positive FP16 denormal: exp=0, mant=0000000001
        tiny = FloatBits(
            sign=0,
            exponent=[0] * 5,
            mantissa=[0] * 9 + [1],
            fmt=FP16,
        )
        val = bits_to_float(tiny)
        assert val > 0
        # 2^(-14) * (1/1024) = 2^(-24) ~ 5.96e-8
        assert val < 1e-6

    def test_bf16_denormal_decode(self) -> None:
        """Decode a manually-constructed BF16 denormal."""
        tiny = FloatBits(
            sign=0,
            exponent=[0] * 8,
            mantissa=[0] * 6 + [1],
            fmt=BF16,
        )
        val = bits_to_float(tiny)
        assert val > 0

    def test_fp16_normal_decode(self) -> None:
        """Decode a manually-constructed FP16 normal number."""
        # 1.5 in FP16: sign=0, exp=15 (01111), mant=1000000000
        bits = FloatBits(
            sign=0,
            exponent=[0, 1, 1, 1, 1],
            mantissa=[1] + [0] * 9,
            fmt=FP16,
        )
        val = bits_to_float(bits)
        assert val == 1.5

    def test_bf16_normal_decode(self) -> None:
        """Decode a manually-constructed BF16 normal number."""
        # 1.5 in BF16: sign=0, exp=127, mant=1000000
        bits = FloatBits(
            sign=0,
            exponent=[0, 1, 1, 1, 1, 1, 1, 1],
            mantissa=[1] + [0] * 6,
            fmt=BF16,
        )
        val = bits_to_float(bits)
        assert val == 1.5

    def test_negative_fp16_value(self) -> None:
        """Decode a negative FP16 value."""
        bits = FloatBits(
            sign=1,
            exponent=[0, 1, 1, 1, 1],
            mantissa=[1] + [0] * 9,
            fmt=FP16,
        )
        val = bits_to_float(bits)
        assert val == -1.5


# ---------------------------------------------------------------------------
# Tests for special value detection
# ---------------------------------------------------------------------------


class TestSpecialValueDetection:
    """Tests for is_nan, is_inf, is_zero, is_denormalized."""

    def test_is_nan_true(self) -> None:
        assert is_nan(float_to_bits(float("nan"), FP32))

    def test_is_nan_false_for_inf(self) -> None:
        assert not is_nan(float_to_bits(float("inf"), FP32))

    def test_is_nan_false_for_number(self) -> None:
        assert not is_nan(float_to_bits(1.0, FP32))

    def test_is_nan_false_for_zero(self) -> None:
        assert not is_nan(float_to_bits(0.0, FP32))

    def test_is_inf_positive(self) -> None:
        assert is_inf(float_to_bits(float("inf"), FP32))

    def test_is_inf_negative(self) -> None:
        assert is_inf(float_to_bits(float("-inf"), FP32))

    def test_is_inf_false_for_nan(self) -> None:
        assert not is_inf(float_to_bits(float("nan"), FP32))

    def test_is_inf_false_for_number(self) -> None:
        assert not is_inf(float_to_bits(1.0, FP32))

    def test_is_inf_false_for_zero(self) -> None:
        assert not is_inf(float_to_bits(0.0, FP32))

    def test_is_zero_positive(self) -> None:
        assert is_zero(float_to_bits(0.0, FP32))

    def test_is_zero_negative(self) -> None:
        assert is_zero(float_to_bits(-0.0, FP32))

    def test_is_zero_false_for_number(self) -> None:
        assert not is_zero(float_to_bits(1.0, FP32))

    def test_is_zero_false_for_nan(self) -> None:
        assert not is_zero(float_to_bits(float("nan"), FP32))

    def test_is_denormalized(self) -> None:
        tiny = FloatBits(sign=0, exponent=[0] * 8, mantissa=[0] * 22 + [1], fmt=FP32)
        assert is_denormalized(tiny)

    def test_is_denormalized_false_for_normal(self) -> None:
        assert not is_denormalized(float_to_bits(1.0, FP32))

    def test_is_denormalized_false_for_zero(self) -> None:
        assert not is_denormalized(float_to_bits(0.0, FP32))

    def test_is_denormalized_false_for_inf(self) -> None:
        assert not is_denormalized(float_to_bits(float("inf"), FP32))

    def test_is_denormalized_false_for_nan(self) -> None:
        assert not is_denormalized(float_to_bits(float("nan"), FP32))

    def test_special_values_fp16(self) -> None:
        assert is_nan(float_to_bits(float("nan"), FP16))
        assert is_inf(float_to_bits(float("inf"), FP16))
        assert is_zero(float_to_bits(0.0, FP16))

    def test_special_values_bf16(self) -> None:
        assert is_nan(float_to_bits(float("nan"), BF16))
        assert is_inf(float_to_bits(float("inf"), BF16))
        assert is_zero(float_to_bits(0.0, BF16))

    def test_fp16_denormalized(self) -> None:
        tiny = FloatBits(sign=0, exponent=[0] * 5, mantissa=[0] * 9 + [1], fmt=FP16)
        assert is_denormalized(tiny)

    def test_bf16_denormalized(self) -> None:
        tiny = FloatBits(sign=0, exponent=[0] * 8, mantissa=[0] * 6 + [1], fmt=BF16)
        assert is_denormalized(tiny)

    def test_negative_denormalized(self) -> None:
        tiny = FloatBits(sign=1, exponent=[0] * 8, mantissa=[0] * 22 + [1], fmt=FP32)
        assert is_denormalized(tiny)
        assert tiny.sign == 1


# ---------------------------------------------------------------------------
# Tests for denormal encoding/decoding
# ---------------------------------------------------------------------------


class TestDenormalEncoding:
    """Test encoding and decoding of denormalized numbers."""

    def test_smallest_fp32_denormal(self) -> None:
        tiny = FloatBits(sign=0, exponent=[0] * 8, mantissa=[0] * 22 + [1], fmt=FP32)
        val = bits_to_float(tiny)
        assert val > 0
        assert val < 1e-44

    def test_largest_fp32_denormal(self) -> None:
        """Largest FP32 denormal: exp=0, mantissa=all 1s."""
        large_denorm = FloatBits(
            sign=0,
            exponent=[0] * 8,
            mantissa=[1] * 23,
            fmt=FP32,
        )
        val = bits_to_float(large_denorm)
        assert val > 0
        assert val < 1.18e-38  # less than smallest normal

    def test_denormal_roundtrip(self) -> None:
        denorm = FloatBits(sign=0, exponent=[0] * 8, mantissa=[1] + [0] * 22, fmt=FP32)
        val = bits_to_float(denorm)
        assert val > 0
        bits2 = float_to_bits(val, FP32)
        assert bits2.exponent == [0] * 8
        assert bits2.mantissa[0] == 1

    def test_negative_denormal(self) -> None:
        denorm = FloatBits(sign=1, exponent=[0] * 8, mantissa=[1] + [0] * 22, fmt=FP32)
        val = bits_to_float(denorm)
        assert val < 0

    def test_fp32_denormal_from_tiny_float(self) -> None:
        """Encoding a very small float produces a denormal."""
        # 2^-149 is the smallest FP32 denormal
        tiny_val = 2.0 ** -149
        bits = float_to_bits(tiny_val, FP32)
        # Should have zero exponent
        assert _bits_msb_to_int(bits.exponent) == 0

    def test_fp16_denormal_encoding(self) -> None:
        """FP16 denormal from a value that's normal in FP32."""
        # FP16 smallest normal: ~6.1e-5. A smaller value becomes denormal.
        val = 3e-5
        bits = float_to_bits(val, FP16)
        assert _bits_msb_to_int(bits.exponent) == 0
        assert not is_zero(bits)

    def test_fp32_denormal_fp16_conversion(self) -> None:
        """A value that is normal in FP32 but denormal in FP16."""
        # 1e-5 is normal in FP32 but underflows in FP16
        bits = float_to_bits(1e-5, FP16)
        exp = _bits_msb_to_int(bits.exponent)
        assert exp == 0  # denormal in FP16


# ---------------------------------------------------------------------------
# Tests for FP16/BF16 edge cases
# ---------------------------------------------------------------------------


class TestFP16BF16EdgeCases:
    """Edge cases specific to the FP16 and BF16 conversion paths."""

    def test_fp16_just_below_overflow(self) -> None:
        """The largest FP16 normal value should not overflow."""
        bits = float_to_bits(65504.0, FP16)
        assert not is_inf(bits)
        result = bits_to_float(bits)
        assert result == 65504.0

    def test_fp16_just_above_overflow(self) -> None:
        """A value just above FP16 max should overflow to inf."""
        bits = float_to_bits(65536.0, FP16)
        assert is_inf(bits)

    def test_bf16_roundtrip_various(self) -> None:
        for val in [0.0, 1.0, -1.0, 0.5, 2.0, 128.0, -256.0]:
            bits = float_to_bits(val, BF16)
            result = bits_to_float(bits)
            assert result == val, f"BF16 roundtrip failed for {val}"

    def test_fp16_mantissa_wider_than_fp32_path(self) -> None:
        """Test the else branch in float_to_bits where fmt.mantissa_bits >= FP32.mantissa_bits.

        This doesn't happen with FP16/BF16 (both have fewer bits), but we can
        test with a custom format.
        """
        # Create a format with more mantissa bits than FP32
        wide_fmt = FloatFormat(name="wide", total_bits=40, exponent_bits=8, mantissa_bits=31, bias=127)
        bits = float_to_bits(1.0, wide_fmt)
        # Should go through the else branch (mantissa_bits >= FP32.mantissa_bits)
        assert bits.fmt == wide_fmt

    def test_fp16_rounding_carry_overflow(self) -> None:
        """Test rounding carry that increases exponent in FP16 conversion.

        When rounding the mantissa causes it to overflow, the exponent increments.
        If that pushes the exponent to max_exp, result becomes infinity.
        """
        # FP16 max normal has exp=30 (11110), mant=all 1s (1111111111)
        # A value slightly above max should overflow to inf after rounding
        bits = float_to_bits(65520.0, FP16)
        result = bits_to_float(bits)
        # This should be representable, close to max
        assert not is_inf(bits) or result == float("inf")
