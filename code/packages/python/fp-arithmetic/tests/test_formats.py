"""Tests for formats.py — FloatFormat and FloatBits data structures."""

import pytest

from fp_arithmetic.formats import BF16, FP16, FP32, FloatBits, FloatFormat


class TestFloatFormat:
    """Tests for the FloatFormat dataclass."""

    def test_fp32_constants(self) -> None:
        assert FP32.name == "fp32"
        assert FP32.total_bits == 32
        assert FP32.exponent_bits == 8
        assert FP32.mantissa_bits == 23
        assert FP32.bias == 127

    def test_fp16_constants(self) -> None:
        assert FP16.name == "fp16"
        assert FP16.total_bits == 16
        assert FP16.exponent_bits == 5
        assert FP16.mantissa_bits == 10
        assert FP16.bias == 15

    def test_bf16_constants(self) -> None:
        assert BF16.name == "bf16"
        assert BF16.total_bits == 16
        assert BF16.exponent_bits == 8
        assert BF16.mantissa_bits == 7
        assert BF16.bias == 127

    def test_fp32_bit_counts_add_up(self) -> None:
        """1 (sign) + exponent + mantissa = total_bits."""
        assert 1 + FP32.exponent_bits + FP32.mantissa_bits == FP32.total_bits

    def test_fp16_bit_counts_add_up(self) -> None:
        assert 1 + FP16.exponent_bits + FP16.mantissa_bits == FP16.total_bits

    def test_bf16_bit_counts_add_up(self) -> None:
        assert 1 + BF16.exponent_bits + BF16.mantissa_bits == BF16.total_bits

    def test_frozen(self) -> None:
        """FloatFormat should be immutable (frozen=True)."""
        with pytest.raises(AttributeError):
            FP32.bias = 42  # type: ignore[misc]

    def test_custom_format(self) -> None:
        """Can create custom formats for testing."""
        custom = FloatFormat(name="fp8", total_bits=8, exponent_bits=4, mantissa_bits=3, bias=7)
        assert custom.total_bits == 8
        assert custom.name == "fp8"
        assert custom.exponent_bits == 4
        assert custom.mantissa_bits == 3
        assert custom.bias == 7

    def test_equality(self) -> None:
        """Two FloatFormats with same values should be equal."""
        fp32_copy = FloatFormat("fp32", 32, 8, 23, 127)
        assert fp32_copy == FP32

    def test_inequality(self) -> None:
        """Different formats should not be equal."""
        assert FP32 != FP16
        assert FP16 != BF16
        assert FP32 != BF16

    def test_bf16_same_exponent_as_fp32(self) -> None:
        """BF16 shares FP32's exponent range."""
        assert BF16.exponent_bits == FP32.exponent_bits
        assert BF16.bias == FP32.bias


class TestFloatBits:
    """Tests for the FloatBits dataclass."""

    def test_create_positive_one(self) -> None:
        """1.0 in FP32: sign=0, exp=01111111, mant=all zeros."""
        bits = FloatBits(
            sign=0,
            exponent=[0, 1, 1, 1, 1, 1, 1, 1],
            mantissa=[0] * 23,
            fmt=FP32,
        )
        assert bits.sign == 0
        assert bits.exponent == [0, 1, 1, 1, 1, 1, 1, 1]
        assert len(bits.mantissa) == 23

    def test_create_negative_one(self) -> None:
        bits = FloatBits(
            sign=1,
            exponent=[0, 1, 1, 1, 1, 1, 1, 1],
            mantissa=[0] * 23,
            fmt=FP32,
        )
        assert bits.sign == 1

    def test_frozen(self) -> None:
        """FloatBits should be immutable."""
        bits = FloatBits(sign=0, exponent=[0] * 8, mantissa=[0] * 23, fmt=FP32)
        with pytest.raises(AttributeError):
            bits.sign = 1  # type: ignore[misc]

    def test_format_reference(self) -> None:
        bits = FloatBits(sign=0, exponent=[0] * 5, mantissa=[0] * 10, fmt=FP16)
        assert bits.fmt == FP16

    def test_bf16_floatbits(self) -> None:
        bits = FloatBits(sign=0, exponent=[0] * 8, mantissa=[0] * 7, fmt=BF16)
        assert bits.fmt == BF16
        assert len(bits.exponent) == 8
        assert len(bits.mantissa) == 7

    def test_create_zero(self) -> None:
        bits = FloatBits(sign=0, exponent=[0] * 8, mantissa=[0] * 23, fmt=FP32)
        assert all(b == 0 for b in bits.exponent)
        assert all(b == 0 for b in bits.mantissa)

    def test_create_inf(self) -> None:
        bits = FloatBits(sign=0, exponent=[1] * 8, mantissa=[0] * 23, fmt=FP32)
        assert all(b == 1 for b in bits.exponent)
        assert all(b == 0 for b in bits.mantissa)

    def test_create_nan(self) -> None:
        bits = FloatBits(sign=0, exponent=[1] * 8, mantissa=[1] + [0] * 22, fmt=FP32)
        assert bits.mantissa[0] == 1
