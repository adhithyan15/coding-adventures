# frozen_string_literal: true

# ============================================================================
# Tests for Adder Circuits
# ============================================================================
#
# We test each adder exhaustively:
# - Half adder: all 4 input combinations (2 bits x 2 bits)
# - Full adder: all 8 input combinations (2 bits x 2 bits x carry)
# - Ripple-carry adder: various multi-bit additions, overflow, edge cases

require_relative "test_helper"

module CodingAdventures
  module Arithmetic
    # -----------------------------------------------------------------------
    # Helpers for converting between integers and LSB-first bit arrays
    # -----------------------------------------------------------------------

    # Converts an integer to an LSB-first bit array of the given width.
    #
    # Example: int_to_bits(5, 4) => [1, 0, 1, 0]
    #   5 in binary is 0101, LSB-first is [1, 0, 1, 0]
    def self.int_to_bits(n, width)
      (0...width).map { |i| (n >> i) & 1 }
    end

    # Converts an LSB-first bit array back to an integer.
    #
    # Example: bits_to_int([1, 0, 1, 0]) => 5
    def self.bits_to_int(bits)
      bits.each_with_index.sum { |bit, i| bit << i }
    end

    # =====================================================================
    # Half Adder Tests
    # =====================================================================

    class TestHalfAdder < Minitest::Test
      # Test every possible input combination (there are only 4).
      # This is an exhaustive test — we verify the complete truth table.

      def test_0_plus_0
        result = Arithmetic.half_adder(0, 0)
        assert_equal 0, result.sum
        assert_equal 0, result.carry
      end

      def test_0_plus_1
        result = Arithmetic.half_adder(0, 1)
        assert_equal 1, result.sum
        assert_equal 0, result.carry
      end

      def test_1_plus_0
        result = Arithmetic.half_adder(1, 0)
        assert_equal 1, result.sum
        assert_equal 0, result.carry
      end

      def test_1_plus_1
        # 1 + 1 = 10 in binary: sum is 0, carry is 1.
        result = Arithmetic.half_adder(1, 1)
        assert_equal 0, result.sum
        assert_equal 1, result.carry
      end

      def test_returns_adder_result
        result = Arithmetic.half_adder(0, 0)
        assert_instance_of AdderResult, result
      end

      def test_result_is_frozen
        # Data.define instances are immutable by default.
        result = Arithmetic.half_adder(1, 0)
        assert result.frozen?
      end
    end

    # =====================================================================
    # Full Adder Tests
    # =====================================================================

    class TestFullAdder < Minitest::Test
      # Test all 8 input combinations (a, b, carry_in each 0 or 1).
      # This is the complete truth table for a full adder.

      def test_0_0_0
        result = Arithmetic.full_adder(0, 0, 0)
        assert_equal 0, result.sum
        assert_equal 0, result.carry
      end

      def test_0_0_1
        result = Arithmetic.full_adder(0, 0, 1)
        assert_equal 1, result.sum
        assert_equal 0, result.carry
      end

      def test_0_1_0
        result = Arithmetic.full_adder(0, 1, 0)
        assert_equal 1, result.sum
        assert_equal 0, result.carry
      end

      def test_0_1_1
        # 0 + 1 + 1 = 10 in binary: sum=0, carry=1
        result = Arithmetic.full_adder(0, 1, 1)
        assert_equal 0, result.sum
        assert_equal 1, result.carry
      end

      def test_1_0_0
        result = Arithmetic.full_adder(1, 0, 0)
        assert_equal 1, result.sum
        assert_equal 0, result.carry
      end

      def test_1_0_1
        # 1 + 0 + 1 = 10 in binary: sum=0, carry=1
        result = Arithmetic.full_adder(1, 0, 1)
        assert_equal 0, result.sum
        assert_equal 1, result.carry
      end

      def test_1_1_0
        # 1 + 1 + 0 = 10 in binary: sum=0, carry=1
        result = Arithmetic.full_adder(1, 1, 0)
        assert_equal 0, result.sum
        assert_equal 1, result.carry
      end

      def test_1_1_1
        # 1 + 1 + 1 = 11 in binary: sum=1, carry=1
        result = Arithmetic.full_adder(1, 1, 1)
        assert_equal 1, result.sum
        assert_equal 1, result.carry
      end

      def test_returns_adder_result
        result = Arithmetic.full_adder(0, 0, 0)
        assert_instance_of AdderResult, result
      end
    end

    # =====================================================================
    # Ripple-Carry Adder Tests
    # =====================================================================

    class TestRippleCarryAdder < Minitest::Test
      def test_0_plus_0
        a = [0, 0, 0, 0]
        b = [0, 0, 0, 0]
        result = Arithmetic.ripple_carry_adder(a, b)
        assert_equal 0, Arithmetic.bits_to_int(result.bits)
        assert_equal 0, result.carry
      end

      def test_1_plus_2
        # The canonical example: x = 1 + 2 = 3
        a = Arithmetic.int_to_bits(1, 4) # [1, 0, 0, 0]
        b = Arithmetic.int_to_bits(2, 4) # [0, 1, 0, 0]
        result = Arithmetic.ripple_carry_adder(a, b)
        assert_equal 3, Arithmetic.bits_to_int(result.bits)
        assert_equal 0, result.carry
      end

      def test_5_plus_3
        a = Arithmetic.int_to_bits(5, 4)
        b = Arithmetic.int_to_bits(3, 4)
        result = Arithmetic.ripple_carry_adder(a, b)
        assert_equal 8, Arithmetic.bits_to_int(result.bits)
        assert_equal 0, result.carry
      end

      def test_15_plus_1_overflow
        # 4-bit overflow: 15 + 1 = 16, which doesn't fit in 4 bits.
        # The result wraps around to 0 with a carry-out of 1.
        a = Arithmetic.int_to_bits(15, 4) # [1, 1, 1, 1]
        b = Arithmetic.int_to_bits(1, 4)  # [1, 0, 0, 0]
        result = Arithmetic.ripple_carry_adder(a, b)
        assert_equal 0, Arithmetic.bits_to_int(result.bits)
        assert_equal 1, result.carry
      end

      def test_with_carry_in
        a = Arithmetic.int_to_bits(1, 4)
        b = Arithmetic.int_to_bits(1, 4)
        result = Arithmetic.ripple_carry_adder(a, b, carry_in: 1)
        assert_equal 3, Arithmetic.bits_to_int(result.bits) # 1 + 1 + carry = 3
        assert_equal 0, result.carry
      end

      def test_8_bit_addition
        a = Arithmetic.int_to_bits(100, 8)
        b = Arithmetic.int_to_bits(155, 8)
        result = Arithmetic.ripple_carry_adder(a, b)
        assert_equal 255, Arithmetic.bits_to_int(result.bits)
        assert_equal 0, result.carry
      end

      def test_8_bit_overflow
        # 255 + 1 = 256 overflows 8 bits
        a = Arithmetic.int_to_bits(255, 8)
        b = Arithmetic.int_to_bits(1, 8)
        result = Arithmetic.ripple_carry_adder(a, b)
        assert_equal 0, Arithmetic.bits_to_int(result.bits)
        assert_equal 1, result.carry
      end

      def test_1_bit_addition
        # Single-bit addition: effectively a full adder
        result = Arithmetic.ripple_carry_adder([1], [1])
        assert_equal [0], result.bits
        assert_equal 1, result.carry
      end

      def test_returns_ripple_carry_result
        result = Arithmetic.ripple_carry_adder([0], [0])
        assert_instance_of RippleCarryResult, result
      end

      # --- Error cases ---

      def test_mismatched_lengths
        error = assert_raises(ArgumentError) do
          Arithmetic.ripple_carry_adder([0, 1], [0, 1, 0])
        end
        assert_match(/same length/, error.message)
      end

      def test_empty_bits
        error = assert_raises(ArgumentError) do
          Arithmetic.ripple_carry_adder([], [])
        end
        assert_match(/must not be empty/, error.message)
      end
    end
  end
end
