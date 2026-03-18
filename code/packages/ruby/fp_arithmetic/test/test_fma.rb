# frozen_string_literal: true

require_relative "test_helper"

class TestFMA < Minitest::Test
  FPA = CodingAdventures::FpArithmetic
  FP32 = FPA::FP32
  FP16 = FPA::FP16
  BF16 = FPA::BF16

  def fp_fma_floats(a, b, c, fmt = FP32)
    bits_a = FPA.float_to_bits(a, fmt)
    bits_b = FPA.float_to_bits(b, fmt)
    bits_c = FPA.float_to_bits(c, fmt)
    result = FPA.fp_fma(bits_a, bits_b, bits_c)
    FPA.bits_to_float(result)
  end

  # --- Basic FMA ---

  def test_fma_2_times_3_plus_1
    # 2 * 3 + 1 = 7
    assert_in_delta 7.0, fp_fma_floats(2.0, 3.0, 1.0), 1e-6
  end

  def test_fma_1_5_times_2_plus_0_25
    # 1.5 * 2.0 + 0.25 = 3.25
    assert_in_delta 3.25, fp_fma_floats(1.5, 2.0, 0.25), 1e-6
  end

  def test_fma_negative_product_plus_positive
    # (-2) * 3 + 10 = -6 + 10 = 4
    assert_in_delta 4.0, fp_fma_floats(-2.0, 3.0, 10.0), 1e-6
  end

  def test_fma_positive_product_minus_result
    # 2 * 3 + (-7) = 6 - 7 = -1
    assert_in_delta(-1.0, fp_fma_floats(2.0, 3.0, -7.0), 1e-6)
  end

  def test_fma_zero_addend
    # 4 * 5 + 0 = 20
    assert_in_delta 20.0, fp_fma_floats(4.0, 5.0, 0.0), 1e-6
  end

  # --- Special values ---

  def test_fma_nan_propagation
    assert fp_fma_floats(Float::NAN, 1.0, 1.0).nan?
    assert fp_fma_floats(1.0, Float::NAN, 1.0).nan?
    assert fp_fma_floats(1.0, 1.0, Float::NAN).nan?
  end

  def test_fma_inf_times_zero_is_nan
    assert fp_fma_floats(Float::INFINITY, 0.0, 1.0).nan?
    assert fp_fma_floats(0.0, Float::INFINITY, 1.0).nan?
  end

  def test_fma_inf_times_finite
    assert_equal Float::INFINITY, fp_fma_floats(Float::INFINITY, 2.0, 1.0)
  end

  def test_fma_inf_product_plus_neg_inf_is_nan
    assert fp_fma_floats(Float::INFINITY, 1.0, -Float::INFINITY).nan?
  end

  def test_fma_zero_product_returns_c
    bits_a = FPA.float_to_bits(0.0)
    bits_b = FPA.float_to_bits(1.0)
    bits_c = FPA.float_to_bits(42.0)
    result = FPA.fp_fma(bits_a, bits_b, bits_c)
    assert_in_delta 42.0, FPA.bits_to_float(result), 1e-6
  end

  def test_fma_c_is_inf
    bits_a = FPA.float_to_bits(2.0)
    bits_b = FPA.float_to_bits(3.0)
    bits_c = FPA.float_to_bits(Float::INFINITY)
    result = FPA.fp_fma(bits_a, bits_b, bits_c)
    assert_equal Float::INFINITY, FPA.bits_to_float(result)
  end

  # --- Format conversion ---

  def test_convert_fp32_to_fp16
    fp32_val = FPA.float_to_bits(1.0, FP32)
    fp16_val = FPA.fp_convert(fp32_val, FP16)
    assert_equal FP16, fp16_val.fmt
    assert_in_delta 1.0, FPA.bits_to_float(fp16_val), 1e-5
  end

  def test_convert_fp32_to_bf16
    fp32_val = FPA.float_to_bits(3.14, FP32)
    bf16_val = FPA.fp_convert(fp32_val, BF16)
    assert_equal BF16, bf16_val.fmt
    result = FPA.bits_to_float(bf16_val)
    assert_in_delta 3.14, result, 0.1
  end

  def test_convert_same_format_returns_same
    fp32_val = FPA.float_to_bits(42.0, FP32)
    result = FPA.fp_convert(fp32_val, FP32)
    assert_equal fp32_val, result
  end

  def test_convert_fp16_to_fp32
    fp16_val = FPA.float_to_bits(2.0, FP16)
    fp32_val = FPA.fp_convert(fp16_val, FP32)
    assert_equal FP32, fp32_val.fmt
    assert_in_delta 2.0, FPA.bits_to_float(fp32_val), 1e-5
  end

  def test_convert_nan
    nan_bits = FPA.float_to_bits(Float::NAN, FP32)
    result = FPA.fp_convert(nan_bits, FP16)
    assert FPA.nan?(result)
  end

  def test_convert_inf
    inf_bits = FPA.float_to_bits(Float::INFINITY, FP32)
    result = FPA.fp_convert(inf_bits, BF16)
    assert FPA.inf?(result)
  end

  # --- Additional FMA edge cases ---

  def test_fma_cancellation_to_zero
    # a * b - a * b = 0
    a = FPA.float_to_bits(3.0)
    b = FPA.float_to_bits(4.0)
    c = FPA.float_to_bits(-12.0)
    result = FPA.fp_fma(a, b, c)
    assert_in_delta 0.0, FPA.bits_to_float(result), 1e-6
  end

  def test_fma_zero_product_zero_c
    a = FPA.float_to_bits(0.0)
    b = FPA.float_to_bits(0.0)
    c = FPA.float_to_bits(0.0)
    result = FPA.fp_fma(a, b, c)
    assert FPA.zero?(result)
  end

  def test_fma_large_product_small_c
    # Large multiply + small addition
    assert_in_delta 1001.0, fp_fma_floats(100.0, 10.0, 1.0), 1e-3
  end

  def test_fma_small_product_large_c
    # Small multiply + large addition
    assert_in_delta 1000.5, fp_fma_floats(0.5, 1.0, 1000.0), 1.0
  end

  def test_fma_negative_c_larger_than_product
    # Product < |c|, result should be negative
    assert_in_delta(-90.0, fp_fma_floats(2.0, 5.0, -100.0), 1e-3)
  end

  def test_fma_inf_product_plus_inf_same_sign
    result = fp_fma_floats(Float::INFINITY, 1.0, Float::INFINITY)
    assert_equal Float::INFINITY, result
  end

  def test_convert_zero
    zero_bits = FPA.float_to_bits(0.0, FP32)
    result = FPA.fp_convert(zero_bits, FP16)
    assert FPA.zero?(result)
    assert_equal FP16, result.fmt
  end

  def test_convert_bf16_to_fp16
    bf16_val = FPA.float_to_bits(2.0, BF16)
    fp16_val = FPA.fp_convert(bf16_val, FP16)
    assert_equal FP16, fp16_val.fmt
    assert_in_delta 2.0, FPA.bits_to_float(fp16_val), 0.01
  end
end
