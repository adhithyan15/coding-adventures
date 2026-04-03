# frozen_string_literal: true

require "minitest/autorun"
require_relative "../lib/gf256"

class TestGF256 < Minitest::Test
  # -------------------------------------------------------------------------
  # Constants
  # -------------------------------------------------------------------------

  def test_zero_is_0
    assert_equal 0, GF256::ZERO
  end

  def test_one_is_1
    assert_equal 1, GF256::ONE
  end

  def test_primitive_polynomial
    assert_equal 0x11D, GF256::PRIMITIVE_POLYNOMIAL
    assert_equal 285, GF256::PRIMITIVE_POLYNOMIAL
  end

  # -------------------------------------------------------------------------
  # Log/Antilog Tables
  # -------------------------------------------------------------------------

  def test_alog_has_256_entries
    assert_equal 256, GF256.alog_table.length
  end

  def test_log_has_256_entries
    assert_equal 256, GF256.log_table.length
  end

  def test_alog_0_is_1
    assert_equal 1, GF256.alog_table[0]
  end

  def test_alog_1_is_2
    assert_equal 2, GF256.alog_table[1]
  end

  def test_alog_8_is_29
    assert_equal 29, GF256.alog_table[8]
  end

  def test_alog_values_in_range
    GF256.alog_table.each do |v|
      assert v >= 1 && v <= 255
    end
  end

  def test_alog_is_bijection
    # ALOG[0..254] are all distinct non-zero values; ALOG[255]=1 is a repeat
    assert_equal 255, GF256.alog_table[0, 255].uniq.length
    refute GF256.alog_table[0, 255].include?(0)
  end

  def test_alog_log_roundtrip
    (1..255).each do |x|
      assert_equal x, GF256.alog_table[GF256.log_table[x]]
    end
  end

  def test_log_alog_roundtrip
    (0..254).each do |i|
      assert_equal i, GF256.log_table[GF256.alog_table[i]]
    end
  end

  def test_log_1_is_0
    assert_equal 0, GF256.log_table[1]
  end

  def test_log_2_is_1
    assert_equal 1, GF256.log_table[2]
  end

  # -------------------------------------------------------------------------
  # add
  # -------------------------------------------------------------------------

  def test_add_zero_is_identity
    (0..255).each do |x|
      assert_equal x, GF256.add(0, x)
      assert_equal x, GF256.add(x, 0)
    end
  end

  def test_add_self_is_zero
    (0..255).each { |x| assert_equal 0, GF256.add(x, x) }
  end

  def test_add_commutative
    (0..31).each do |x|
      (0..31).each do |y|
        assert_equal GF256.add(x, y), GF256.add(y, x)
      end
    end
  end

  def test_add_is_xor
    (0..255).each do |x|
      assert_equal x ^ 0x42, GF256.add(x, 0x42)
    end
  end

  # -------------------------------------------------------------------------
  # subtract
  # -------------------------------------------------------------------------

  def test_subtract_self_is_zero
    (0..255).each { |x| assert_equal 0, GF256.subtract(x, x) }
  end

  def test_subtract_equals_add
    (0..31).each do |x|
      (0..31).each do |y|
        assert_equal GF256.add(x, y), GF256.subtract(x, y)
      end
    end
  end

  # -------------------------------------------------------------------------
  # multiply
  # -------------------------------------------------------------------------

  def test_multiply_by_zero
    (0..255).each do |x|
      assert_equal 0, GF256.multiply(x, 0)
      assert_equal 0, GF256.multiply(0, x)
    end
  end

  def test_multiply_by_one
    (0..255).each do |x|
      assert_equal x, GF256.multiply(x, 1)
      assert_equal x, GF256.multiply(1, x)
    end
  end

  def test_multiply_commutative
    (0..31).each do |x|
      (0..31).each do |y|
        assert_equal GF256.multiply(x, y), GF256.multiply(y, x)
      end
    end
  end

  def test_spot_check_0x53_times_0x8C_is_1
    # With primitive polynomial 0x11D: inverse(0x53) = 0x8C
    assert_equal 1, GF256.multiply(0x53, 0x8C)
  end

  def test_multiply_distributive_over_add
    a, b, c = 0x34, 0x56, 0x78
    assert_equal(
      GF256.add(GF256.multiply(a, b), GF256.multiply(a, c)),
      GF256.multiply(a, GF256.add(b, c))
    )
  end

  # -------------------------------------------------------------------------
  # divide
  # -------------------------------------------------------------------------

  def test_divide_by_zero_raises
    assert_raises(ArgumentError) { GF256.divide(1, 0) }
    assert_raises(ArgumentError) { GF256.divide(0, 0) }
  end

  def test_divide_by_one
    (0..255).each { |x| assert_equal x, GF256.divide(x, 1) }
  end

  def test_divide_zero_by_anything
    (1..255).each { |x| assert_equal 0, GF256.divide(0, x) }
  end

  def test_divide_self_is_one
    (1..255).each { |x| assert_equal 1, GF256.divide(x, x) }
  end

  def test_divide_is_inverse_of_multiply
    (0..15).each do |a|
      (1..15).each do |b|
        assert_equal a, GF256.divide(GF256.multiply(a, b), b)
      end
    end
  end

  # -------------------------------------------------------------------------
  # power
  # -------------------------------------------------------------------------

  def test_any_nonzero_to_zero_is_one
    (1..255).each { |x| assert_equal 1, GF256.power(x, 0) }
  end

  def test_zero_to_zero_is_one
    assert_equal 1, GF256.power(0, 0)
  end

  def test_zero_to_positive_is_zero
    assert_equal 0, GF256.power(0, 1)
    assert_equal 0, GF256.power(0, 5)
  end

  def test_generator_order_255
    assert_equal 1, GF256.power(2, 255)
  end

  def test_power_matches_alog
    (0..254).each do |i|
      assert_equal GF256.alog_table[i], GF256.power(2, i)
    end
  end

  # -------------------------------------------------------------------------
  # inverse
  # -------------------------------------------------------------------------

  def test_inverse_zero_raises
    assert_raises(ArgumentError) { GF256.inverse(0) }
  end

  def test_inverse_one_is_one
    assert_equal 1, GF256.inverse(1)
  end

  def test_inverse_times_self_is_one
    (1..255).each do |x|
      assert_equal 1, GF256.multiply(x, GF256.inverse(x))
    end
  end

  def test_inverse_of_inverse_is_self
    (1..255).each do |x|
      assert_equal x, GF256.inverse(GF256.inverse(x))
    end
  end

  def test_spot_check_0x53_inverse_is_0x8C
    # With primitive polynomial 0x11D: inverse(0x53) = 0x8C
    assert_equal 0x8C, GF256.inverse(0x53)
  end

  # -------------------------------------------------------------------------
  # zero and one
  # -------------------------------------------------------------------------

  def test_zero_method_returns_0
    assert_equal 0, GF256.zero
  end

  def test_one_method_returns_1
    assert_equal 1, GF256.one
  end

  def test_zero_is_additive_identity
    assert_equal 0x42, GF256.add(GF256.zero, 0x42)
    assert_equal 0x42, GF256.add(0x42, GF256.zero)
  end

  def test_one_is_multiplicative_identity
    assert_equal 0x42, GF256.multiply(GF256.one, 0x42)
    assert_equal 0x42, GF256.multiply(0x42, GF256.one)
  end
end
