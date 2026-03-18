# frozen_string_literal: true

require_relative "test_helper"

class TestFPMultiplier < Minitest::Test
  FPA = CodingAdventures::FpArithmetic
  FP32 = FPA::FP32

  def fp_mul_floats(a, b, fmt = FP32)
    bits_a = FPA.float_to_bits(a, fmt)
    bits_b = FPA.float_to_bits(b, fmt)
    result = FPA.fp_mul(bits_a, bits_b)
    FPA.bits_to_float(result)
  end

  # --- Basic multiplication ---

  def test_mul_one_times_one
    assert_in_delta 1.0, fp_mul_floats(1.0, 1.0), 1e-6
  end

  def test_mul_two_times_three
    assert_in_delta 6.0, fp_mul_floats(2.0, 3.0), 1e-6
  end

  def test_mul_1_5_times_2
    assert_in_delta 3.0, fp_mul_floats(1.5, 2.0), 1e-6
  end

  def test_mul_negative_times_positive
    assert_in_delta(-6.0, fp_mul_floats(-2.0, 3.0), 1e-6)
  end

  def test_mul_negative_times_negative
    assert_in_delta 6.0, fp_mul_floats(-2.0, -3.0), 1e-6
  end

  def test_mul_by_half
    assert_in_delta 2.5, fp_mul_floats(5.0, 0.5), 1e-6
  end

  def test_mul_large_numbers
    assert_in_delta 1_000_000.0, fp_mul_floats(1000.0, 1000.0), 10.0
  end

  # --- Special values ---

  def test_mul_nan_propagation
    assert fp_mul_floats(Float::NAN, 1.0).nan?
    assert fp_mul_floats(1.0, Float::NAN).nan?
  end

  def test_mul_inf_times_zero_is_nan
    assert fp_mul_floats(Float::INFINITY, 0.0).nan?
    assert fp_mul_floats(0.0, Float::INFINITY).nan?
  end

  def test_mul_inf_times_finite
    assert_equal Float::INFINITY, fp_mul_floats(Float::INFINITY, 2.0)
    assert_equal(-Float::INFINITY, fp_mul_floats(Float::INFINITY, -2.0))
  end

  def test_mul_inf_times_inf
    assert_equal Float::INFINITY, fp_mul_floats(Float::INFINITY, Float::INFINITY)
  end

  def test_mul_zero_times_anything
    assert_equal 0.0, fp_mul_floats(0.0, 42.0)
    assert_equal 0.0, fp_mul_floats(42.0, 0.0)
  end

  def test_mul_sign_of_zero_result
    bits_a = FPA.float_to_bits(-0.0)
    bits_b = FPA.float_to_bits(1.0)
    result = FPA.fp_mul(bits_a, bits_b)
    # -0 * 1 should give sign = 1 (negative)
    assert_equal 1, result.sign
  end

  # --- Overflow ---

  def test_mul_overflow_to_inf
    huge = FPA.float_to_bits(3.4e38, FP32)
    result = FPA.fp_mul(huge, huge)
    assert FPA.inf?(result)
  end

  def test_mul_denormal_times_normal
    denorm = FPA::FloatBits.new(
      sign: 0,
      exponent: Array.new(8, 0),
      mantissa: [1] + Array.new(22, 0),
      fmt: FP32
    )
    normal = FPA.float_to_bits(2.0)
    result = FPA.fp_mul(denorm, normal)
    val = FPA.bits_to_float(result)
    assert val >= 0
  end

  def test_mul_very_small_numbers_underflow_to_zero
    tiny = FPA.float_to_bits(1e-30, FP32)
    result = FPA.fp_mul(tiny, tiny)
    val = FPA.bits_to_float(result)
    # Product of two very small numbers should be zero or denormal
    assert val >= 0.0
    assert val < 1e-38
  end

  def test_mul_one_times_anything
    [0.5, 2.0, -3.0, 100.0].each do |v|
      result = fp_mul_floats(1.0, v)
      assert_in_delta v, result, (v.abs * 1e-6) + 1e-10, "1.0 * #{v} failed"
    end
  end

  def test_mul_inf_times_neg_inf
    assert_equal(-Float::INFINITY, fp_mul_floats(Float::INFINITY, -Float::INFINITY))
  end
end
