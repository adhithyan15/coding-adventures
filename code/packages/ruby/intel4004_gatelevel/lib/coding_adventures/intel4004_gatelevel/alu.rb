# frozen_string_literal: true

# ---------------------------------------------------------------------------
# 4-bit ALU -- the arithmetic heart of the Intel 4004.
# ---------------------------------------------------------------------------
#
# === How the real 4004's ALU worked ===
#
# The Intel 4004 had a 4-bit ALU that could add, subtract, and perform
# logical operations on 4-bit values. It used a ripple-carry adder built
# from full adders, which were themselves built from AND, OR, and XOR gates.
#
# This module wraps the arithmetic package's ALU(bit_width: 4) to provide
# the exact operations the 4004 needs. Every addition and subtraction
# physically routes through the gate chain:
#
#     XOR -> AND -> OR -> full_adder -> ripple_carry_adder -> ALU
#
# That is real hardware simulation -- not behavioral shortcuts.
#
# === Subtraction via complement-add ===
#
# The 4004 does not have a dedicated subtractor. Instead, it uses the
# ones' complement method:
#
#     A - B = A + NOT(B) + borrow_in
#
# where borrow_in = 0 if carry_flag else 1 (inverted carry semantics).
# The ALU's NOT operation does this internally using NOT gates to
# complement B, then feeding through the same adder.
# ---------------------------------------------------------------------------

require "coding_adventures_arithmetic"

module CodingAdventures
  module Intel4004Gatelevel
    class GateALU
      # All operations route through real logic gates via the arithmetic
      # package's ALU class. No behavioral shortcuts.
      #
      # The ALU provides:
      #   - add(a, b, carry_in) -> [result, carry_out]
      #   - subtract(a, b, borrow_in) -> [result, carry_out]
      #   - complement(a) -> result (4-bit NOT)
      #   - increment(a) -> [result, carry_out]
      #   - decrement(a) -> [result, borrow_out]

      def initialize
        @alu = Arithmetic::ALU.new(bit_width: 4)
      end

      # Add two 4-bit values with carry.
      #
      # Routes through: XOR -> AND -> OR -> full_adder x 4 -> ripple_carry
      #
      # @param a [Integer] first operand (0-15)
      # @param b [Integer] second operand (0-15)
      # @param carry_in [Integer] carry from previous operation (0 or 1)
      # @return [Array(Integer, Boolean)] [result, carry_out] where result is 4-bit (0-15)
      def add(a, b, carry_in = 0)
        a_bits = Bits.int_to_bits(a, 4)
        b_bits = Bits.int_to_bits(b, 4)

        if carry_in != 0
          # Add carry_in by first adding a+b, then adding 1
          # This simulates the carry input to the LSB full adder
          result1 = @alu.execute(Arithmetic::ALUOp::ADD, a_bits, b_bits)
          one_bits = Bits.int_to_bits(1, 4)
          result2 = @alu.execute(Arithmetic::ALUOp::ADD, result1.result, one_bits)
          # Carry is set if either addition overflowed
          carry = result1.carry || result2.carry
          [Bits.bits_to_int(result2.result), carry]
        else
          result = @alu.execute(Arithmetic::ALUOp::ADD, a_bits, b_bits)
          [Bits.bits_to_int(result.result), result.carry]
        end
      end

      # Subtract using complement-add: A + NOT(B) + borrow_in.
      #
      # The 4004's carry flag semantics for subtraction:
      #     carry=true  -> no borrow (result >= 0)
      #     carry=false -> borrow occurred
      #
      # @param a [Integer] minuend (0-15)
      # @param b [Integer] subtrahend (0-15)
      # @param borrow_in [Integer] 1 if no previous borrow, 0 if borrow
      # @return [Array(Integer, Boolean)] [result, carry_out] where carry_out=true means no borrow
      def subtract(a, b, borrow_in = 0)
        # Complement b using NOT gates
        b_bits = Bits.int_to_bits(b, 4)
        b_comp = @alu.execute(Arithmetic::ALUOp::NOT, b_bits, b_bits)
        # A + NOT(B) + borrow_in
        add(a, Bits.bits_to_int(b_comp.result), borrow_in)
      end

      # 4-bit NOT: invert all bits using NOT gates.
      #
      # @param a [Integer] value to complement (0-15)
      # @return [Integer] complemented value (0-15)
      def complement(a)
        a_bits = Bits.int_to_bits(a, 4)
        result = @alu.execute(Arithmetic::ALUOp::NOT, a_bits, a_bits)
        Bits.bits_to_int(result.result)
      end

      # Increment by 1 using the adder. Returns [result, carry].
      def increment(a)
        add(a, 1, 0)
      end

      # Decrement by 1 using complement-add.
      #
      # A - 1 = A + NOT(1) + 1 = A + 14 + 1 = A + 15.
      # carry=true if A > 0 (no borrow), false if A == 0.
      def decrement(a)
        subtract(a, 1, 1)
      end

      # 4-bit AND using AND gates.
      def bitwise_and(a, b)
        a_bits = Bits.int_to_bits(a, 4)
        b_bits = Bits.int_to_bits(b, 4)
        result = @alu.execute(Arithmetic::ALUOp::AND, a_bits, b_bits)
        Bits.bits_to_int(result.result)
      end

      # 4-bit OR using OR gates.
      def bitwise_or(a, b)
        a_bits = Bits.int_to_bits(a, 4)
        b_bits = Bits.int_to_bits(b, 4)
        result = @alu.execute(Arithmetic::ALUOp::OR, a_bits, b_bits)
        Bits.bits_to_int(result.result)
      end

      # Estimated gate count for a 4-bit ALU.
      #
      # Each full adder: 5 gates (2 XOR + 2 AND + 1 OR).
      # 4-bit ripple carry: 4 x 5 = 20 gates.
      # SUB complement: 4 NOT gates.
      # Control muxing: ~8 gates.
      # Total: ~32 gates.
      def gate_count
        32
      end
    end
  end
end
