# frozen_string_literal: true

require_relative "test_helper"

class TestFormats < Minitest::Test
  FP32 = CodingAdventures::FpArithmetic::FP32
  FP16 = CodingAdventures::FpArithmetic::FP16
  BF16 = CodingAdventures::FpArithmetic::BF16
  FloatFormat = CodingAdventures::FpArithmetic::FloatFormat
  FloatBits = CodingAdventures::FpArithmetic::FloatBits

  # --- FloatFormat tests ---

  def test_fp32_properties
    assert_equal "fp32", FP32.name
    assert_equal 32, FP32.total_bits
    assert_equal 8, FP32.exponent_bits
    assert_equal 23, FP32.mantissa_bits
    assert_equal 127, FP32.bias
  end

  def test_fp16_properties
    assert_equal "fp16", FP16.name
    assert_equal 16, FP16.total_bits
    assert_equal 5, FP16.exponent_bits
    assert_equal 10, FP16.mantissa_bits
    assert_equal 15, FP16.bias
  end

  def test_bf16_properties
    assert_equal "bf16", BF16.name
    assert_equal 16, BF16.total_bits
    assert_equal 8, BF16.exponent_bits
    assert_equal 7, BF16.mantissa_bits
    assert_equal 127, BF16.bias
  end

  def test_float_format_is_immutable
    assert_raises(FrozenError) { FP32.instance_variable_set(:@name, "modified") }
  end

  def test_float_format_equality
    other = FloatFormat.new(name: "fp32", total_bits: 32, exponent_bits: 8, mantissa_bits: 23, bias: 127)
    assert_equal FP32, other
  end

  # --- FloatBits tests ---

  def test_float_bits_creation
    bits = FloatBits.new(
      sign: 0,
      exponent: [0, 1, 1, 1, 1, 1, 1, 1],
      mantissa: Array.new(23, 0),
      fmt: FP32
    )
    assert_equal 0, bits.sign
    assert_equal [0, 1, 1, 1, 1, 1, 1, 1], bits.exponent
    assert_equal Array.new(23, 0), bits.mantissa
    assert_equal FP32, bits.fmt
  end

  def test_float_bits_is_immutable
    bits = FloatBits.new(sign: 0, exponent: [0] * 8, mantissa: [0] * 23, fmt: FP32)
    assert_raises(FrozenError) { bits.instance_variable_set(:@sign, 1) }
  end

  def test_float_bits_equality
    a = FloatBits.new(sign: 0, exponent: [1] * 8, mantissa: [0] * 23, fmt: FP32)
    b = FloatBits.new(sign: 0, exponent: [1] * 8, mantissa: [0] * 23, fmt: FP32)
    assert_equal a, b
  end
end
