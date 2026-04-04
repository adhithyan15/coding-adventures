# frozen_string_literal: true

# --------------------------------------------------------------------------
# gf256_native_test.rb — Tests for the Rust-backed GF256Native module
# --------------------------------------------------------------------------
#
# These tests exercise every public function and constant of the native GF(2^8)
# extension to ensure the Rust implementation is correctly exposed to Ruby.
#
# GF(2^8) has 256 elements (0..=255). Key facts:
# - addition = subtraction = XOR (characteristic 2)
# - multiplication uses log/antilog tables modulo the primitive polynomial 0x11D
# - every non-zero element has a multiplicative inverse

require_relative "test_helper"

M = CodingAdventures::GF256Native

class GF256NativeTest < Minitest::Test
  # ========================================================================
  # Module constants
  # ========================================================================

  def test_zero_constant
    assert_equal 0, M::ZERO
  end

  def test_one_constant
    assert_equal 1, M::ONE
  end

  def test_primitive_polynomial_constant
    # 0x11D = x^8 + x^4 + x^3 + x^2 + 1 = 285
    assert_equal 0x11D, M::PRIMITIVE_POLYNOMIAL
    assert_equal 285, M::PRIMITIVE_POLYNOMIAL
  end

  # ========================================================================
  # add tests — addition is XOR in characteristic-2 field
  # ========================================================================

  def test_add_basic_xor
    # 0x53 XOR 0xCA = 0x99
    assert_equal 0x99, M.add(0x53, 0xCA)
  end

  def test_add_is_xor
    # Addition in GF(2^8) is bitwise XOR
    (0..255).step(17) do |a|
      (0..255).step(13) do |b|
        assert_equal a ^ b, M.add(a, b), "add(#{a}, #{b}) should equal XOR"
      end
    end
  end

  def test_add_zero_is_identity
    # a + 0 = a for all a
    assert_equal 42, M.add(42, 0)
    assert_equal 0, M.add(0, 0)
    assert_equal 255, M.add(255, 0)
  end

  def test_add_is_self_inverse
    # a + a = 0 (every element is its own additive inverse in characteristic 2)
    (0..255).step(7) do |a|
      assert_equal 0, M.add(a, a), "a + a should be 0 for a = #{a}"
    end
  end

  def test_add_is_commutative
    assert_equal M.add(42, 77), M.add(77, 42)
    assert_equal M.add(0, 255), M.add(255, 0)
  end

  def test_add_out_of_range_raises
    assert_raises(ArgumentError) { M.add(256, 0) }
    assert_raises(ArgumentError) { M.add(0, -1) }
  end

  # ========================================================================
  # subtract tests — same as add in characteristic 2
  # ========================================================================

  def test_subtract_equals_add
    # subtract(a, b) = add(a, b) in GF(2^8) because -x = x
    (0..255).step(11) do |a|
      (0..255).step(17) do |b|
        assert_equal M.add(a, b), M.subtract(a, b),
          "subtract(#{a}, #{b}) should equal add(#{a}, #{b})"
      end
    end
  end

  def test_subtract_self_is_zero
    (0..255).step(13) do |a|
      assert_equal 0, M.subtract(a, a), "a - a = 0 for a = #{a}"
    end
  end

  def test_subtract_zero_is_identity
    assert_equal 99, M.subtract(99, 0)
  end

  # ========================================================================
  # multiply tests
  # ========================================================================

  def test_multiply_by_zero
    # 0 × anything = 0
    (0..255).step(7) do |a|
      assert_equal 0, M.multiply(a, 0), "#{a} * 0 = 0"
      assert_equal 0, M.multiply(0, a), "0 * #{a} = 0"
    end
  end

  def test_multiply_by_one_is_identity
    # 1 × a = a
    (0..255).step(7) do |a|
      assert_equal a, M.multiply(a, 1), "1 * #{a} = #{a}"
      assert_equal a, M.multiply(1, a), "#{a} * 1 = #{a}"
    end
  end

  def test_multiply_by_two
    # Multiplying by 2 (the generator g) is shift-left with conditional XOR
    # 2 * 128 = 256 XOR 0x11D = 29
    assert_equal 29, M.multiply(2, 128)
    assert_equal 29, M.multiply(128, 2)
  end

  def test_multiply_generator_powers
    # 2^8 should be ALOG[8] = 29
    assert_equal 29, M.power(2, 8)
    # Check via multiply: 2 * (2^7) = 2 * 128 = 29
    assert_equal 29, M.multiply(2, M.power(2, 7))
  end

  def test_multiply_is_commutative
    assert_equal M.multiply(53, 202), M.multiply(202, 53)
    assert_equal M.multiply(255, 128), M.multiply(128, 255)
  end

  def test_multiply_out_of_range_raises
    assert_raises(ArgumentError) { M.multiply(256, 1) }
    assert_raises(ArgumentError) { M.multiply(1, -1) }
  end

  # ========================================================================
  # divide tests
  # ========================================================================

  def test_divide_by_one_is_identity
    (0..255).step(7) do |a|
      assert_equal a, M.divide(a, 1), "#{a} / 1 = #{a}"
    end
  end

  def test_divide_zero_by_anything
    # 0 / x = 0 for all nonzero x
    (1..255).step(7) do |b|
      assert_equal 0, M.divide(0, b), "0 / #{b} = 0"
    end
  end

  def test_divide_by_zero_raises
    assert_raises(ArgumentError) { M.divide(1, 0) }
    assert_raises(ArgumentError) { M.divide(0, 0) }
  end

  def test_divide_self_is_one
    # a / a = 1 for all non-zero a
    (1..255).step(7) do |a|
      assert_equal 1, M.divide(a, a), "#{a} / #{a} = 1"
    end
  end

  def test_divide_undoes_multiply
    # (a * b) / b = a for nonzero b
    (1..255).step(11) do |a|
      (1..255).step(13) do |b|
        product = M.multiply(a, b)
        assert_equal a, M.divide(product, b),
          "multiply then divide: (#{a} * #{b}) / #{b} should be #{a}"
      end
    end
  end

  # ========================================================================
  # power tests
  # ========================================================================

  def test_power_zero_exponent
    # Any nonzero element raised to 0 is 1
    (1..255).step(7) do |a|
      assert_equal 1, M.power(a, 0), "#{a}^0 = 1"
    end
  end

  def test_power_zero_base_zero_exp
    # 0^0 = 1 by convention
    assert_equal 1, M.power(0, 0)
  end

  def test_power_zero_base_positive_exp
    # 0^n = 0 for n > 0
    assert_equal 0, M.power(0, 1)
    assert_equal 0, M.power(0, 255)
  end

  def test_power_by_one
    # a^1 = a
    (0..255).step(7) do |a|
      assert_equal a, M.power(a, 1), "#{a}^1 = #{a}"
    end
  end

  def test_power_generator
    # 2 is the generator. 2^255 = 1 (the multiplicative group has order 255).
    assert_equal 1, M.power(2, 255)
    # 2^1 = 2
    assert_equal 2, M.power(2, 1)
    # 2^8 = 29 (first reduction by 0x11D)
    assert_equal 29, M.power(2, 8)
  end

  def test_power_negative_exp_raises
    assert_raises(ArgumentError) { M.power(2, -1) }
  end

  def test_power_out_of_range_base_raises
    assert_raises(ArgumentError) { M.power(256, 2) }
  end

  # ========================================================================
  # inverse tests
  # ========================================================================

  def test_inverse_one_is_one
    # 1^(-1) = 1 (1 is its own inverse)
    assert_equal 1, M.inverse(1)
  end

  def test_inverse_zero_raises
    # 0 has no multiplicative inverse
    assert_raises(ArgumentError) { M.inverse(0) }
  end

  def test_inverse_times_self_is_one
    # a * inverse(a) = 1 for all nonzero a
    (1..255).step(3) do |a|
      inv = M.inverse(a)
      assert_equal 1, M.multiply(a, inv),
        "#{a} * inverse(#{a}) should be 1 (got #{inv})"
    end
  end

  def test_inverse_of_inverse_is_self
    # inverse(inverse(a)) = a
    (1..255).step(7) do |a|
      assert_equal a, M.inverse(M.inverse(a)),
        "inverse(inverse(#{a})) should be #{a}"
    end
  end

  def test_inverse_known_value
    # From ALOG[255 - LOG[2]] = ALOG[255 - 1] = ALOG[254]
    # 2 * 142 = 1 in GF(256) — a known test vector
    inv2 = M.inverse(2)
    assert_equal 1, M.multiply(2, inv2), "2 * inverse(2) = 1"
  end

  def test_inverse_out_of_range_raises
    assert_raises(ArgumentError) { M.inverse(256) }
    assert_raises(ArgumentError) { M.inverse(-1) }
  end

  # ========================================================================
  # Version constant
  # ========================================================================

  def test_version_constant_exists
    assert_equal "0.1.0", CodingAdventures::GF256Native::VERSION
  end

  # ========================================================================
  # Algebraic identity tests
  # ========================================================================

  def test_distributive_law
    # a * (b + c) = (a * b) + (a * c)
    [[2, 3, 5], [17, 200, 99], [255, 128, 64]].each do |a, b, c|
      lhs = M.multiply(a, M.add(b, c))
      rhs = M.add(M.multiply(a, b), M.multiply(a, c))
      assert_equal lhs, rhs, "distributive law: #{a} * (#{b} + #{c})"
    end
  end

  def test_multiplicative_group_order
    # For any nonzero a, a^255 = 1 (Fermat's little theorem for GF(256))
    [2, 3, 7, 42, 128, 255].each do |a|
      assert_equal 1, M.power(a, 255), "#{a}^255 = 1 in GF(256)"
    end
  end
end
