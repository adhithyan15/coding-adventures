# frozen_string_literal: true

# ============================================================================
# Arithmetic Logic Unit (ALU) — The Computational Heart of a CPU
# ============================================================================
#
# The ALU is the component inside a CPU that performs all calculations. It
# takes two N-bit inputs and an operation code, and produces an N-bit result
# plus several status flags.
#
# Every CPU has an ALU at its core. When you write `x = 1 + 2` in any
# programming language, the instruction eventually reaches the ALU as:
#   "ADD these two bit patterns together."
#
# Our ALU supports six operations:
#
#   ADD — addition via ripple-carry adder
#   SUB — subtraction via two's complement (NOT(b) + 1, then ADD)
#   AND — bitwise AND across all bit positions
#   OR  — bitwise OR across all bit positions
#   XOR — bitwise XOR across all bit positions
#   NOT — bitwise NOT of the first operand (second operand ignored)
#
# Status flags (these are essential for conditional branching):
#
#   zero     — is the result all zeros?
#   carry    — did unsigned addition overflow?
#   negative — is the most significant bit 1? (sign bit in two's complement)
#   overflow — did signed addition overflow?
#
# Two's complement subtraction:
#   A - B = A + NOT(B) + 1
#   This works because NOT(B) + 1 is the two's complement negation of B.
#   Example in 4-bit: 5 - 3
#     B = 0011, NOT(B) = 1100, NOT(B)+1 = 1101 (which is -3 in two's complement)
#     5 + (-3) = 0101 + 1101 = 10010 -> result=0010 (2), carry=1
# ============================================================================

require "coding_adventures_logic_gates"
require_relative "adders"

module CodingAdventures
  module Arithmetic
    # -----------------------------------------------------------------------
    # ALU operation codes
    # -----------------------------------------------------------------------

    # ALUOp defines the operations the ALU can perform.
    #
    # In a real CPU, the operation code comes from the instruction decoder,
    # which translates machine code into control signals. Here we use
    # Ruby symbols for clarity.
    module ALUOp
      ADD = :add
      SUB = :sub
      AND = :and
      OR  = :or
      XOR = :xor
      NOT = :not

      # All valid operations, for validation.
      ALL = [ADD, SUB, AND, OR, XOR, NOT].freeze
    end

    # -----------------------------------------------------------------------
    # ALU result
    # -----------------------------------------------------------------------

    # ALUResult holds the output of an ALU operation.
    #
    # - result:   Array of N bits (LSB-first), the computed value
    # - zero:     true if every bit in result is 0
    # - carry:    true if unsigned addition overflowed (carry-out = 1)
    # - negative: true if the most significant bit is 1 (sign bit)
    # - overflow: true if signed overflow occurred
    ALUResult = Data.define(:result, :zero, :carry, :negative, :overflow)

    # -----------------------------------------------------------------------
    # ALU class
    # -----------------------------------------------------------------------

    # An N-bit Arithmetic Logic Unit.
    #
    # The ALU is parameterized by bit_width, which determines how many bits
    # each operand and result has. Common widths in real hardware:
    # - 8-bit:  early microprocessors (Intel 8080, MOS 6502)
    # - 16-bit: IBM PC era (Intel 8086)
    # - 32-bit: modern embedded systems (ARM Cortex-M)
    # - 64-bit: modern desktop/server CPUs (x86-64, ARM64)
    class ALU
      # @return [Integer] the number of bits this ALU operates on
      attr_reader :bit_width

      # Creates a new ALU with the given bit width.
      #
      # @param bit_width [Integer] number of bits per operand (must be >= 1)
      # @raise [ArgumentError] if bit_width is less than 1
      def initialize(bit_width: 8)
        if bit_width < 1
          raise ArgumentError, "bit_width must be at least 1"
        end

        @bit_width = bit_width
      end

      # Executes an ALU operation on two N-bit operands.
      #
      # @param op [Symbol] the operation (one of ALUOp constants)
      # @param a [Array<Integer>] first operand as LSB-first bit array
      # @param b [Array<Integer>] second operand as LSB-first bit array
      #   (ignored for NOT operation)
      # @return [ALUResult] the result with status flags
      # @raise [ArgumentError] if operands have wrong bit width or op is unknown
      #
      # @example Adding 1 + 2 on an 8-bit ALU
      #   alu = CodingAdventures::Arithmetic::ALU.new(bit_width: 8)
      #   a = [1, 0, 0, 0, 0, 0, 0, 0]  # 1 in 8-bit LSB-first
      #   b = [0, 1, 0, 0, 0, 0, 0, 0]  # 2 in 8-bit LSB-first
      #   result = alu.execute(ALUOp::ADD, a, b)
      #   result.result  # => [1, 1, 0, 0, 0, 0, 0, 0]  (3)
      #   result.zero    # => false
      #   result.carry   # => false
      def execute(op, a, b)
        validate_operands!(op, a, b)

        carry = false

        case op
        when ALUOp::ADD
          rca = Arithmetic.ripple_carry_adder(a, b)
          value = rca.bits
          carry = rca.carry == 1

        when ALUOp::SUB
          # A - B = A + NOT(B) + 1 (two's complement subtraction)
          neg_b = twos_complement_negate(b)
          rca = Arithmetic.ripple_carry_adder(a, neg_b)
          value = rca.bits
          carry = rca.carry == 1

        when ALUOp::AND
          value = bitwise_op(a, b) { |x, y| LogicGates.and_gate(x, y) }

        when ALUOp::OR
          value = bitwise_op(a, b) { |x, y| LogicGates.or_gate(x, y) }

        when ALUOp::XOR
          value = bitwise_op(a, b) { |x, y| LogicGates.xor_gate(x, y) }

        when ALUOp::NOT
          value = a.map { |bit| LogicGates.not_gate(bit) }

        else
          raise ArgumentError, "Unknown operation: #{op.inspect}"
        end

        # Compute status flags
        zero = value.all? { |bit| bit == 0 }
        negative = value.empty? ? false : value[-1] == 1

        # Signed overflow detection:
        # Overflow occurs when adding two numbers of the same sign produces
        # a result of the opposite sign. This happens because the result is
        # too large (or too negative) to fit in N bits of two's complement.
        #
        # For subtraction (A - B), we check the sign of the effective second
        # operand, which is NOT(B) since A - B = A + NOT(B) + 1.
        overflow_flag = false
        if [ALUOp::ADD, ALUOp::SUB].include?(op)
          a_sign = a[-1]
          b_sign = (op == ALUOp::ADD) ? b[-1] : LogicGates.not_gate(b[-1])
          result_sign = value[-1]
          overflow_flag = (a_sign == b_sign) && (result_sign != a_sign)
        end

        ALUResult.new(
          result: value,
          zero: zero,
          carry: carry,
          negative: negative,
          overflow: overflow_flag
        )
      end

      private

      # Validates operand lengths match the ALU's bit width.
      def validate_operands!(op, a, b)
        if a.length != @bit_width
          raise ArgumentError, "a must have #{@bit_width} bits, got #{a.length}"
        end
        if op != ALUOp::NOT && b.length != @bit_width
          raise ArgumentError, "b must have #{@bit_width} bits, got #{b.length}"
        end
      end

      # Applies a 2-input gate bitwise across two bit arrays.
      #
      # @param a [Array<Integer>] first bit array
      # @param b [Array<Integer>] second bit array
      # @yield [Integer, Integer] called with corresponding bits from a and b
      # @return [Array<Integer>] result bit array
      def bitwise_op(a, b, &block)
        a.zip(b).map { |x, y| block.call(x, y) }
      end

      # Negates a number using two's complement: NOT(bits) + 1.
      #
      # Two's complement is the standard way computers represent negative
      # numbers. To negate a number:
      #   1. Flip all bits (NOT)
      #   2. Add 1
      #
      # Example: negating 3 in 4-bit
      #   3 = 0011
      #   NOT(3) = 1100
      #   NOT(3) + 1 = 1101 = -3 in two's complement
      #
      # @param bits [Array<Integer>] the bits to negate (LSB-first)
      # @return [Array<Integer>] the negated bits (LSB-first)
      def twos_complement_negate(bits)
        inverted = bits.map { |b| LogicGates.not_gate(b) }
        one = [1] + Array.new(bits.length - 1, 0)
        result = Arithmetic.ripple_carry_adder(inverted, one)
        result.bits
      end
    end
  end
end
