# frozen_string_literal: true

require_relative "test_helper"

class TestIEEE754 < Minitest::Test
  FPA = CodingAdventures::FpArithmetic
  FP32 = FPA::FP32
  FP16 = FPA::FP16
  BF16 = FPA::BF16

  # --- int_to_bits_msb / bits_msb_to_int ---

  def test_int_to_bits_msb_zero
    assert_equal [0, 0, 0, 0], FPA.int_to_bits_msb(0, 4)
  end

  def test_int_to_bits_msb_five
    assert_equal [0, 0, 0, 0, 0, 1, 0, 1], FPA.int_to_bits_msb(5, 8)
  end

  def test_int_to_bits_msb_max_8bit
    assert_equal [1, 1, 1, 1, 1, 1, 1, 1], FPA.int_to_bits_msb(255, 8)
  end

  def test_bits_msb_to_int_roundtrip
    (0..15).each do |val|
      bits = FPA.int_to_bits_msb(val, 4)
      assert_equal val, FPA.bits_msb_to_int(bits)
    end
  end

  # --- float_to_bits / bits_to_float FP32 ---

  def test_encode_decode_one
    bits = FPA.float_to_bits(1.0, FP32)
    assert_equal 0, bits.sign
    assert_equal [0, 1, 1, 1, 1, 1, 1, 1], bits.exponent  # 127
    assert_equal Array.new(23, 0), bits.mantissa
    assert_in_delta 1.0, FPA.bits_to_float(bits), 1e-10
  end

  def test_encode_decode_negative_one
    bits = FPA.float_to_bits(-1.0, FP32)
    assert_equal 1, bits.sign
    assert_in_delta(-1.0, FPA.bits_to_float(bits), 1e-10)
  end

  def test_encode_decode_pi
    bits = FPA.float_to_bits(3.14, FP32)
    assert_equal 0, bits.sign
    result = FPA.bits_to_float(bits)
    assert_in_delta 3.14, result, 0.001
  end

  def test_encode_decode_zero
    bits = FPA.float_to_bits(0.0, FP32)
    assert_equal 0, bits.sign
    assert_equal 0.0, FPA.bits_to_float(bits)
  end

  def test_encode_decode_negative_zero
    bits = FPA.float_to_bits(-0.0, FP32)
    assert_equal 1, bits.sign
    result = FPA.bits_to_float(bits)
    assert_equal 0.0, result  # -0.0 == 0.0 in Ruby
  end

  def test_encode_decode_nan
    bits = FPA.float_to_bits(Float::NAN, FP32)
    assert FPA.bits_to_float(bits).nan?
  end

  def test_encode_decode_positive_inf
    bits = FPA.float_to_bits(Float::INFINITY, FP32)
    assert_equal Float::INFINITY, FPA.bits_to_float(bits)
  end

  def test_encode_decode_negative_inf
    bits = FPA.float_to_bits(-Float::INFINITY, FP32)
    assert_equal(-Float::INFINITY, FPA.bits_to_float(bits))
  end

  def test_encode_decode_small_number
    bits = FPA.float_to_bits(0.5, FP32)
    assert_in_delta 0.5, FPA.bits_to_float(bits), 1e-10
  end

  def test_encode_decode_large_number
    bits = FPA.float_to_bits(1_000_000.0, FP32)
    assert_in_delta 1_000_000.0, FPA.bits_to_float(bits), 1.0
  end

  # --- FP16 encoding/decoding ---

  def test_fp16_one
    bits = FPA.float_to_bits(1.0, FP16)
    assert_equal 0, bits.sign
    result = FPA.bits_to_float(bits)
    assert_in_delta 1.0, result, 1e-5
  end

  def test_fp16_negative
    bits = FPA.float_to_bits(-2.0, FP16)
    assert_equal 1, bits.sign
    assert_in_delta(-2.0, FPA.bits_to_float(bits), 1e-5)
  end

  def test_fp16_zero
    bits = FPA.float_to_bits(0.0, FP16)
    assert_equal 0.0, FPA.bits_to_float(bits)
  end

  def test_fp16_inf
    bits = FPA.float_to_bits(Float::INFINITY, FP16)
    assert_equal Float::INFINITY, FPA.bits_to_float(bits)
  end

  def test_fp16_nan
    bits = FPA.float_to_bits(Float::NAN, FP16)
    assert FPA.bits_to_float(bits).nan?
  end

  def test_fp16_overflow_to_inf
    # FP16 max is about 65504; a larger value should become Inf
    bits = FPA.float_to_bits(100_000.0, FP16)
    assert_equal Float::INFINITY, FPA.bits_to_float(bits)
  end

  # --- BF16 encoding/decoding ---

  def test_bf16_one
    bits = FPA.float_to_bits(1.0, BF16)
    assert_equal 0, bits.sign
    result = FPA.bits_to_float(bits)
    assert_in_delta 1.0, result, 1e-3
  end

  def test_bf16_pi
    bits = FPA.float_to_bits(3.14, BF16)
    result = FPA.bits_to_float(bits)
    assert_in_delta 3.14, result, 0.1
  end

  def test_bf16_zero
    bits = FPA.float_to_bits(0.0, BF16)
    assert_equal 0.0, FPA.bits_to_float(bits)
  end

  def test_bf16_nan
    bits = FPA.float_to_bits(Float::NAN, BF16)
    assert FPA.bits_to_float(bits).nan?
  end

  # --- Special value detection ---

  def test_is_nan
    bits = FPA.float_to_bits(Float::NAN, FP32)
    assert FPA.nan?(bits)
    refute FPA.inf?(bits)
    refute FPA.zero?(bits)
    refute FPA.denormalized?(bits)
  end

  def test_is_inf
    bits = FPA.float_to_bits(Float::INFINITY, FP32)
    assert FPA.inf?(bits)
    refute FPA.nan?(bits)
    refute FPA.zero?(bits)
  end

  def test_is_negative_inf
    bits = FPA.float_to_bits(-Float::INFINITY, FP32)
    assert FPA.inf?(bits)
    assert_equal 1, bits.sign
  end

  def test_is_zero
    bits = FPA.float_to_bits(0.0, FP32)
    assert FPA.zero?(bits)
    refute FPA.nan?(bits)
    refute FPA.inf?(bits)
  end

  def test_is_negative_zero
    bits = FPA.float_to_bits(-0.0, FP32)
    assert FPA.zero?(bits)
    assert_equal 1, bits.sign
  end

  def test_is_denormalized
    # The smallest positive FP32 denormal: exponent=0, mantissa=[0]*22 + [1]
    tiny = CodingAdventures::FpArithmetic::FloatBits.new(
      sign: 0,
      exponent: Array.new(8, 0),
      mantissa: Array.new(22, 0) + [1],
      fmt: FP32
    )
    assert FPA.denormalized?(tiny)
    refute FPA.zero?(tiny)
    refute FPA.nan?(tiny)
    refute FPA.inf?(tiny)
  end

  def test_normal_number_is_not_special
    bits = FPA.float_to_bits(42.0, FP32)
    refute FPA.nan?(bits)
    refute FPA.inf?(bits)
    refute FPA.zero?(bits)
    refute FPA.denormalized?(bits)
  end

  # --- all_ones? / all_zeros? ---

  def test_all_ones
    assert FPA.all_ones?([1, 1, 1, 1])
    refute FPA.all_ones?([1, 0, 1, 1])
    refute FPA.all_ones?([0, 0, 0, 0])
  end

  def test_all_zeros
    assert FPA.all_zeros?([0, 0, 0, 0])
    refute FPA.all_zeros?([0, 0, 1, 0])
    refute FPA.all_zeros?([1, 1, 1, 1])
  end

  # --- FP16 denormal encoding ---

  def test_fp16_very_small_becomes_denormal_or_zero
    # A very tiny FP32 number that underflows to zero in FP16
    bits = FPA.float_to_bits(1e-10, FP16)
    assert FPA.zero?(bits)
  end

  def test_fp16_negative_zero
    bits = FPA.float_to_bits(-0.0, FP16)
    assert_equal 1, bits.sign
    assert FPA.zero?(bits)
  end

  def test_bf16_negative_inf
    bits = FPA.float_to_bits(-Float::INFINITY, BF16)
    assert FPA.inf?(bits)
    assert_equal 1, bits.sign
  end

  def test_bf16_negative_zero
    bits = FPA.float_to_bits(-0.0, BF16)
    assert_equal 1, bits.sign
    assert FPA.zero?(bits)
  end

  def test_fp16_denormal_value
    # FP16 smallest denormal ~ 5.96e-8
    # A value just above that should encode as denormal
    tiny = FPA::FloatBits.new(
      sign: 0,
      exponent: Array.new(5, 0),
      mantissa: Array.new(9, 0) + [1],
      fmt: FP16
    )
    assert FPA.denormalized?(tiny)
    val = FPA.bits_to_float(tiny)
    assert val > 0
    assert val < 1e-4
  end

  def test_bf16_denormal_value
    tiny = FPA::FloatBits.new(
      sign: 0,
      exponent: Array.new(8, 0),
      mantissa: Array.new(6, 0) + [1],
      fmt: BF16
    )
    assert FPA.denormalized?(tiny)
    val = FPA.bits_to_float(tiny)
    assert val > 0
  end

  def test_fp16_roundtrip_various_values
    [0.5, 1.0, 2.0, 100.0, 0.001].each do |v|
      bits = FPA.float_to_bits(v, FP16)
      result = FPA.bits_to_float(bits)
      assert_in_delta v, result, v * 0.01 + 0.001, "FP16 roundtrip failed for #{v}"
    end
  end

  def test_bf16_roundtrip_various_values
    [0.5, 1.0, 2.0, 100.0, 1e10].each do |v|
      bits = FPA.float_to_bits(v, BF16)
      result = FPA.bits_to_float(bits)
      assert_in_delta v, result, v * 0.05 + 0.01, "BF16 roundtrip failed for #{v}"
    end
  end

  def test_fp32_denormal_to_fp16_underflow
    # FP32 denormal is way too small for FP16, should become zero
    denorm = FPA::FloatBits.new(
      sign: 0,
      exponent: Array.new(8, 0),
      mantissa: Array.new(22, 0) + [1],
      fmt: FP32
    )
    val = FPA.bits_to_float(denorm)
    fp16 = FPA.float_to_bits(val, FP16)
    assert FPA.zero?(fp16)
  end
end
