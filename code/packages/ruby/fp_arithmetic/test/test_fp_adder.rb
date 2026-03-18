# frozen_string_literal: true

require_relative "test_helper"

class TestFPAdder < Minitest::Test
  FPA = CodingAdventures::FpArithmetic
  FP32 = FPA::FP32

  # Helper: add two Ruby floats using our FP adder and return the result as a float.
  def fp_add_floats(a, b, fmt = FP32)
    bits_a = FPA.float_to_bits(a, fmt)
    bits_b = FPA.float_to_bits(b, fmt)
    result = FPA.fp_add(bits_a, bits_b)
    FPA.bits_to_float(result)
  end

  def fp_sub_floats(a, b, fmt = FP32)
    bits_a = FPA.float_to_bits(a, fmt)
    bits_b = FPA.float_to_bits(b, fmt)
    result = FPA.fp_sub(bits_a, bits_b)
    FPA.bits_to_float(result)
  end

  # --- Basic addition ---

  def test_add_one_plus_one
    assert_in_delta 2.0, fp_add_floats(1.0, 1.0), 1e-6
  end

  def test_add_one_plus_two
    assert_in_delta 3.0, fp_add_floats(1.0, 2.0), 1e-6
  end

  def test_add_1_5_plus_0_25
    assert_in_delta 1.75, fp_add_floats(1.5, 0.25), 1e-6
  end

  def test_add_large_plus_small
    assert_in_delta 1001.0, fp_add_floats(1000.0, 1.0), 1.0
  end

  def test_add_negative_numbers
    assert_in_delta(-3.0, fp_add_floats(-1.0, -2.0), 1e-6)
  end

  def test_add_positive_and_negative
    assert_in_delta(-1.0, fp_add_floats(1.0, -2.0), 1e-6)
  end

  def test_add_cancellation
    assert_in_delta 0.0, fp_add_floats(1.0, -1.0), 1e-6
  end

  # --- Special values ---

  def test_add_nan_propagation
    assert fp_add_floats(Float::NAN, 1.0).nan?
    assert fp_add_floats(1.0, Float::NAN).nan?
    assert fp_add_floats(Float::NAN, Float::NAN).nan?
  end

  def test_add_inf_plus_inf
    assert_equal Float::INFINITY, fp_add_floats(Float::INFINITY, Float::INFINITY)
  end

  def test_add_inf_plus_neg_inf
    assert fp_add_floats(Float::INFINITY, -Float::INFINITY).nan?
  end

  def test_add_inf_plus_finite
    assert_equal Float::INFINITY, fp_add_floats(Float::INFINITY, 1.0)
    assert_equal(-Float::INFINITY, fp_add_floats(-Float::INFINITY, 1.0))
  end

  def test_add_zero_plus_zero
    result = fp_add_floats(0.0, 0.0)
    assert_equal 0.0, result
  end

  def test_add_zero_plus_number
    assert_in_delta 5.0, fp_add_floats(0.0, 5.0), 1e-6
    assert_in_delta 5.0, fp_add_floats(5.0, 0.0), 1e-6
  end

  # --- Subtraction ---

  def test_sub_three_minus_one
    assert_in_delta 2.0, fp_sub_floats(3.0, 1.0), 1e-6
  end

  def test_sub_one_minus_three
    assert_in_delta(-2.0, fp_sub_floats(1.0, 3.0), 1e-6)
  end

  def test_sub_self
    assert_in_delta 0.0, fp_sub_floats(42.0, 42.0), 1e-6
  end

  # --- Negation ---

  def test_neg_positive
    bits = FPA.float_to_bits(5.0)
    neg = FPA.fp_neg(bits)
    assert_in_delta(-5.0, FPA.bits_to_float(neg), 1e-6)
  end

  def test_neg_negative
    bits = FPA.float_to_bits(-3.0)
    neg = FPA.fp_neg(bits)
    assert_in_delta 3.0, FPA.bits_to_float(neg), 1e-6
  end

  def test_neg_zero
    bits = FPA.float_to_bits(0.0)
    neg = FPA.fp_neg(bits)
    assert_equal 1, neg.sign  # -0
  end

  # --- Absolute value ---

  def test_abs_positive
    bits = FPA.float_to_bits(5.0)
    result = FPA.fp_abs(bits)
    assert_equal 0, result.sign
    assert_in_delta 5.0, FPA.bits_to_float(result), 1e-6
  end

  def test_abs_negative
    bits = FPA.float_to_bits(-5.0)
    result = FPA.fp_abs(bits)
    assert_equal 0, result.sign
    assert_in_delta 5.0, FPA.bits_to_float(result), 1e-6
  end

  # --- Comparison ---

  def test_compare_equal
    a = FPA.float_to_bits(1.0)
    b = FPA.float_to_bits(1.0)
    assert_equal 0, FPA.fp_compare(a, b)
  end

  def test_compare_less_than
    a = FPA.float_to_bits(1.0)
    b = FPA.float_to_bits(2.0)
    assert_equal(-1, FPA.fp_compare(a, b))
  end

  def test_compare_greater_than
    a = FPA.float_to_bits(3.0)
    b = FPA.float_to_bits(2.0)
    assert_equal 1, FPA.fp_compare(a, b)
  end

  def test_compare_negative_numbers
    a = FPA.float_to_bits(-1.0)
    b = FPA.float_to_bits(-2.0)
    assert_equal 1, FPA.fp_compare(a, b)  # -1 > -2
  end

  def test_compare_positive_vs_negative
    a = FPA.float_to_bits(1.0)
    b = FPA.float_to_bits(-1.0)
    assert_equal 1, FPA.fp_compare(a, b)
  end

  def test_compare_zeros
    a = FPA.float_to_bits(0.0)
    b = FPA.float_to_bits(-0.0)
    assert_equal 0, FPA.fp_compare(a, b)
  end

  def test_compare_nan_returns_zero
    a = FPA.float_to_bits(Float::NAN)
    b = FPA.float_to_bits(1.0)
    assert_equal 0, FPA.fp_compare(a, b)
    assert_equal 0, FPA.fp_compare(b, a)
  end

  def test_compare_zero_vs_positive
    a = FPA.float_to_bits(0.0)
    b = FPA.float_to_bits(1.0)
    assert_equal(-1, FPA.fp_compare(a, b))
  end

  def test_compare_zero_vs_negative
    a = FPA.float_to_bits(0.0)
    b = FPA.float_to_bits(-1.0)
    assert_equal 1, FPA.fp_compare(a, b)
  end

  # --- Additional edge cases for coverage ---

  def test_add_large_exponent_difference
    # Adding a very small number to a very large one
    assert_in_delta 1e20, fp_add_floats(1e20, 1.0), 1e14
  end

  def test_add_negative_zero_plus_negative_zero
    a = FPA.float_to_bits(-0.0)
    b = FPA.float_to_bits(-0.0)
    result = FPA.fp_add(a, b)
    assert_equal 1, result.sign  # -0 + -0 = -0
  end

  def test_add_positive_zero_plus_negative_zero
    a = FPA.float_to_bits(0.0)
    b = FPA.float_to_bits(-0.0)
    result = FPA.fp_add(a, b)
    assert_equal 0, result.sign  # +0 + -0 = +0
  end

  def test_sub_nan
    assert fp_sub_floats(Float::NAN, 1.0).nan?
  end

  def test_add_same_magnitude_different_signs
    # Near-cancellation test
    assert_in_delta 0.0, fp_add_floats(1.0000001, -1.0000001), 1e-4
  end

  def test_add_overflow_to_inf
    huge = 3.4e38
    result = fp_add_floats(huge, huge)
    assert_equal Float::INFINITY, result
  end

  def test_compare_same_exponent_different_mantissa
    a = FPA.float_to_bits(1.5)
    b = FPA.float_to_bits(1.25)
    assert_equal 1, FPA.fp_compare(a, b)
    assert_equal(-1, FPA.fp_compare(b, a))
  end

  def test_compare_negative_same_exponent_different_mantissa
    a = FPA.float_to_bits(-1.5)
    b = FPA.float_to_bits(-1.25)
    assert_equal(-1, FPA.fp_compare(a, b))  # -1.5 < -1.25
    assert_equal 1, FPA.fp_compare(b, a)
  end

  def test_neg_nan
    bits = FPA.float_to_bits(Float::NAN)
    neg = FPA.fp_neg(bits)
    assert FPA.bits_to_float(neg).nan?
  end

  def test_abs_nan
    bits = FPA.float_to_bits(Float::NAN)
    result = FPA.fp_abs(bits)
    assert_equal 0, result.sign
  end

  def test_add_denormal_plus_normal
    # A denormal + a normal number
    denorm = FPA::FloatBits.new(
      sign: 0,
      exponent: Array.new(8, 0),
      mantissa: [1] + Array.new(22, 0),
      fmt: FP32
    )
    normal = FPA.float_to_bits(1.0)
    result = FPA.fp_add(denorm, normal)
    assert_in_delta 1.0, FPA.bits_to_float(result), 1e-6
  end

  def test_add_two_denormals
    d1 = FPA::FloatBits.new(sign: 0, exponent: Array.new(8, 0),
      mantissa: [1] + Array.new(22, 0), fmt: FP32)
    d2 = FPA::FloatBits.new(sign: 0, exponent: Array.new(8, 0),
      mantissa: [1] + Array.new(22, 0), fmt: FP32)
    result = FPA.fp_add(d1, d2)
    val = FPA.bits_to_float(result)
    assert val > 0
  end
end
