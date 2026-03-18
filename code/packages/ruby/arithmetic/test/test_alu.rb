# frozen_string_literal: true

# ============================================================================
# Tests for the Arithmetic Logic Unit (ALU)
# ============================================================================
#
# We test each ALU operation and all status flags. The ALU is the central
# computation unit, so thorough testing is critical — every higher layer
# depends on it producing correct results.

require_relative "test_helper"

module CodingAdventures
  module Arithmetic
    # =====================================================================
    # ALU Addition Tests
    # =====================================================================

    class TestALUAdd < Minitest::Test
      def setup
        @alu = ALU.new(bit_width: 8)
      end

      def test_1_plus_2
        # The canonical program: x = 1 + 2 = 3
        result = @alu.execute(
          ALUOp::ADD,
          Arithmetic.int_to_bits(1, 8),
          Arithmetic.int_to_bits(2, 8)
        )
        assert_equal 3, Arithmetic.bits_to_int(result.result)
        assert_equal false, result.zero
        assert_equal false, result.carry
      end

      def test_0_plus_0
        result = @alu.execute(
          ALUOp::ADD,
          Arithmetic.int_to_bits(0, 8),
          Arithmetic.int_to_bits(0, 8)
        )
        assert_equal 0, Arithmetic.bits_to_int(result.result)
        assert_equal true, result.zero
      end

      def test_overflow_255_plus_1
        # 255 + 1 = 256, overflows 8 bits: result wraps to 0, carry set
        result = @alu.execute(
          ALUOp::ADD,
          Arithmetic.int_to_bits(255, 8),
          Arithmetic.int_to_bits(1, 8)
        )
        assert_equal 0, Arithmetic.bits_to_int(result.result)
        assert_equal true, result.carry
        assert_equal true, result.zero
      end

      def test_100_plus_50
        result = @alu.execute(
          ALUOp::ADD,
          Arithmetic.int_to_bits(100, 8),
          Arithmetic.int_to_bits(50, 8)
        )
        assert_equal 150, Arithmetic.bits_to_int(result.result)
        assert_equal false, result.carry
      end

      def test_128_plus_128_overflow
        # 128 + 128 = 256 -> wraps to 0 with carry
        result = @alu.execute(
          ALUOp::ADD,
          Arithmetic.int_to_bits(128, 8),
          Arithmetic.int_to_bits(128, 8)
        )
        assert_equal 0, Arithmetic.bits_to_int(result.result)
        assert_equal true, result.carry
      end
    end

    # =====================================================================
    # ALU Subtraction Tests
    # =====================================================================

    class TestALUSub < Minitest::Test
      def setup
        @alu = ALU.new(bit_width: 8)
      end

      def test_5_minus_3
        result = @alu.execute(
          ALUOp::SUB,
          Arithmetic.int_to_bits(5, 8),
          Arithmetic.int_to_bits(3, 8)
        )
        assert_equal 2, Arithmetic.bits_to_int(result.result)
        assert_equal false, result.zero
      end

      def test_3_minus_3
        result = @alu.execute(
          ALUOp::SUB,
          Arithmetic.int_to_bits(3, 8),
          Arithmetic.int_to_bits(3, 8)
        )
        assert_equal 0, Arithmetic.bits_to_int(result.result)
        assert_equal true, result.zero
      end

      def test_10_minus_1
        result = @alu.execute(
          ALUOp::SUB,
          Arithmetic.int_to_bits(10, 8),
          Arithmetic.int_to_bits(1, 8)
        )
        assert_equal 9, Arithmetic.bits_to_int(result.result)
      end

      def test_0_minus_1_wraps
        # Unsigned: 0 - 1 wraps to 255 in 8-bit
        result = @alu.execute(
          ALUOp::SUB,
          Arithmetic.int_to_bits(0, 8),
          Arithmetic.int_to_bits(1, 8)
        )
        assert_equal 255, Arithmetic.bits_to_int(result.result)
        assert_equal true, result.negative
      end
    end

    # =====================================================================
    # ALU Bitwise Operation Tests
    # =====================================================================

    class TestALUBitwise < Minitest::Test
      def setup
        @alu = ALU.new(bit_width: 8)
      end

      def test_and
        # 0b11001100 AND 0b10101010 = 0b10001000
        result = @alu.execute(
          ALUOp::AND,
          Arithmetic.int_to_bits(0xCC, 8),
          Arithmetic.int_to_bits(0xAA, 8)
        )
        assert_equal 0x88, Arithmetic.bits_to_int(result.result)
      end

      def test_or
        # 0b11001100 OR 0b10101010 = 0b11101110
        result = @alu.execute(
          ALUOp::OR,
          Arithmetic.int_to_bits(0xCC, 8),
          Arithmetic.int_to_bits(0xAA, 8)
        )
        assert_equal 0xEE, Arithmetic.bits_to_int(result.result)
      end

      def test_xor
        # 0b11001100 XOR 0b10101010 = 0b01100110
        result = @alu.execute(
          ALUOp::XOR,
          Arithmetic.int_to_bits(0xCC, 8),
          Arithmetic.int_to_bits(0xAA, 8)
        )
        assert_equal 0x66, Arithmetic.bits_to_int(result.result)
      end

      def test_not
        # NOT 0b00000000 = 0b11111111 = 255
        result = @alu.execute(
          ALUOp::NOT,
          Arithmetic.int_to_bits(0, 8),
          []
        )
        assert_equal 255, Arithmetic.bits_to_int(result.result)
      end

      def test_not_of_ff
        # NOT 0xFF = 0x00
        result = @alu.execute(
          ALUOp::NOT,
          Arithmetic.int_to_bits(0xFF, 8),
          []
        )
        assert_equal 0, Arithmetic.bits_to_int(result.result)
        assert_equal true, result.zero
      end

      def test_and_with_zero_produces_zero
        result = @alu.execute(
          ALUOp::AND,
          Arithmetic.int_to_bits(0xFF, 8),
          Arithmetic.int_to_bits(0x00, 8)
        )
        assert_equal 0, Arithmetic.bits_to_int(result.result)
        assert_equal true, result.zero
      end

      def test_or_with_zero_is_identity
        result = @alu.execute(
          ALUOp::OR,
          Arithmetic.int_to_bits(0x42, 8),
          Arithmetic.int_to_bits(0x00, 8)
        )
        assert_equal 0x42, Arithmetic.bits_to_int(result.result)
      end

      def test_xor_with_self_is_zero
        result = @alu.execute(
          ALUOp::XOR,
          Arithmetic.int_to_bits(0xAB, 8),
          Arithmetic.int_to_bits(0xAB, 8)
        )
        assert_equal 0, Arithmetic.bits_to_int(result.result)
        assert_equal true, result.zero
      end
    end

    # =====================================================================
    # ALU Status Flag Tests
    # =====================================================================

    class TestALUFlags < Minitest::Test
      def setup
        @alu = ALU.new(bit_width: 8)
      end

      def test_zero_flag_on_and
        # 0xF0 AND 0x0F = 0x00 -> zero flag set
        result = @alu.execute(
          ALUOp::AND,
          Arithmetic.int_to_bits(0xF0, 8),
          Arithmetic.int_to_bits(0x0F, 8)
        )
        assert_equal true, result.zero
      end

      def test_zero_flag_not_set
        result = @alu.execute(
          ALUOp::ADD,
          Arithmetic.int_to_bits(1, 8),
          Arithmetic.int_to_bits(0, 8)
        )
        assert_equal false, result.zero
      end

      def test_negative_flag
        # MSB set = negative in two's complement
        # 128 = 0b10000000, MSB is 1
        result = @alu.execute(
          ALUOp::ADD,
          Arithmetic.int_to_bits(128, 8),
          Arithmetic.int_to_bits(0, 8)
        )
        assert_equal true, result.negative
      end

      def test_negative_flag_not_set
        result = @alu.execute(
          ALUOp::ADD,
          Arithmetic.int_to_bits(1, 8),
          Arithmetic.int_to_bits(2, 8)
        )
        assert_equal false, result.negative
      end

      def test_signed_overflow_positive
        # 127 + 1 = 128, but in signed 8-bit, 127 + 1 = -128 (overflow!)
        # Two positive numbers produce a "negative" result.
        result = @alu.execute(
          ALUOp::ADD,
          Arithmetic.int_to_bits(127, 8),
          Arithmetic.int_to_bits(1, 8)
        )
        assert_equal true, result.overflow
        assert_equal true, result.negative
      end

      def test_no_overflow_on_normal_add
        result = @alu.execute(
          ALUOp::ADD,
          Arithmetic.int_to_bits(50, 8),
          Arithmetic.int_to_bits(25, 8)
        )
        assert_equal false, result.overflow
      end

      def test_carry_not_set_on_bitwise
        result = @alu.execute(
          ALUOp::AND,
          Arithmetic.int_to_bits(0xFF, 8),
          Arithmetic.int_to_bits(0xFF, 8)
        )
        assert_equal false, result.carry
      end

      def test_overflow_not_set_on_bitwise
        result = @alu.execute(
          ALUOp::OR,
          Arithmetic.int_to_bits(0xFF, 8),
          Arithmetic.int_to_bits(0xFF, 8)
        )
        assert_equal false, result.overflow
      end
    end

    # =====================================================================
    # ALU Validation Tests
    # =====================================================================

    class TestALUValidation < Minitest::Test
      def setup
        @alu = ALU.new(bit_width: 8)
      end

      def test_wrong_bit_width_a
        error = assert_raises(ArgumentError) do
          @alu.execute(ALUOp::ADD, [0, 1], [0, 1])
        end
        assert_match(/8 bits/, error.message)
      end

      def test_wrong_bit_width_b
        error = assert_raises(ArgumentError) do
          @alu.execute(ALUOp::ADD, Arithmetic.int_to_bits(0, 8), [0, 1])
        end
        assert_match(/8 bits/, error.message)
      end

      def test_invalid_bit_width_zero
        error = assert_raises(ArgumentError) do
          ALU.new(bit_width: 0)
        end
        assert_match(/at least 1/, error.message)
      end

      def test_invalid_bit_width_negative
        error = assert_raises(ArgumentError) do
          ALU.new(bit_width: -1)
        end
        assert_match(/at least 1/, error.message)
      end

      def test_unknown_operation
        error = assert_raises(ArgumentError) do
          @alu.execute(:invalid, Arithmetic.int_to_bits(0, 8), Arithmetic.int_to_bits(0, 8))
        end
        assert_match(/Unknown operation/, error.message)
      end

      def test_not_ignores_b
        # NOT should work even with empty b
        result = @alu.execute(ALUOp::NOT, Arithmetic.int_to_bits(0xAA, 8), [])
        assert_equal 0x55, Arithmetic.bits_to_int(result.result)
      end
    end

    # =====================================================================
    # ALU Different Bit Widths
    # =====================================================================

    class TestALUBitWidths < Minitest::Test
      def test_4_bit_alu
        alu = ALU.new(bit_width: 4)
        result = alu.execute(
          ALUOp::ADD,
          Arithmetic.int_to_bits(7, 4),
          Arithmetic.int_to_bits(3, 4)
        )
        assert_equal 10, Arithmetic.bits_to_int(result.result)
        assert_equal false, result.carry
      end

      def test_1_bit_alu
        alu = ALU.new(bit_width: 1)
        result = alu.execute(ALUOp::ADD, [1], [1])
        assert_equal [0], result.result
        assert_equal true, result.carry
      end

      def test_16_bit_alu
        alu = ALU.new(bit_width: 16)
        result = alu.execute(
          ALUOp::ADD,
          Arithmetic.int_to_bits(1000, 16),
          Arithmetic.int_to_bits(2000, 16)
        )
        assert_equal 3000, Arithmetic.bits_to_int(result.result)
      end

      def test_bit_width_accessor
        alu = ALU.new(bit_width: 32)
        assert_equal 32, alu.bit_width
      end
    end

    # =====================================================================
    # ALU Result Data Type Tests
    # =====================================================================

    class TestALUResult < Minitest::Test
      def test_alu_result_is_data
        result = ALUResult.new(
          result: [0, 1, 0, 0],
          zero: false,
          carry: false,
          negative: false,
          overflow: false
        )
        assert_instance_of ALUResult, result
        assert_equal [0, 1, 0, 0], result.result
        assert_equal false, result.zero
      end

      def test_alu_result_is_frozen
        alu = ALU.new(bit_width: 8)
        result = alu.execute(
          ALUOp::ADD,
          Arithmetic.int_to_bits(1, 8),
          Arithmetic.int_to_bits(2, 8)
        )
        assert result.frozen?
      end
    end
  end
end
