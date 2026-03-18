# frozen_string_literal: true

# ============================================================================
# Adder Circuits — Building Addition from Logic Gates
# ============================================================================
#
# How do computers add numbers? Not with the + operator — that is a
# high-level abstraction. At the hardware level, addition is performed by
# circuits made entirely of logic gates (AND, OR, XOR).
#
# We build addition in three stages:
#
# 1. HALF ADDER: adds two single bits.
#    - The sum is XOR (are they different?)
#    - The carry is AND (are they both 1?)
#
# 2. FULL ADDER: adds two bits plus a carry-in from a previous stage.
#    - Built from two half adders and an OR gate.
#    - This is the workhorse — one full adder handles one column of addition.
#
# 3. RIPPLE-CARRY ADDER: chains N full adders together.
#    - Each full adder's carry-out feeds into the next adder's carry-in.
#    - The carry "ripples" from least significant to most significant bit.
#    - This is exactly how you add multi-digit numbers by hand!
#
# Example: adding 5 + 3 in 4-bit binary (LSB first):
#
#   Bit 0: full_adder(1, 1, 0) → sum=0, carry=1   (1+1 = 10 binary)
#   Bit 1: full_adder(0, 1, 1) → sum=0, carry=1   (0+1+carry = 10)
#   Bit 2: full_adder(1, 0, 1) → sum=0, carry=1   (1+0+carry = 10)
#   Bit 3: full_adder(0, 0, 1) → sum=1, carry=0   (0+0+carry = 01)
#   Result: [0, 0, 0, 1] = 8   (5 + 3 = 8)
# ============================================================================

require "coding_adventures_logic_gates"

module CodingAdventures
  module Arithmetic
    # -----------------------------------------------------------------------
    # Return types
    # -----------------------------------------------------------------------

    # AdderResult holds the output of a half adder or full adder.
    #
    # In digital circuits, an adder always produces two outputs:
    # - sum:   the result bit for this column
    # - carry: the overflow bit to pass to the next column
    #
    # We use Data.define (Ruby 3.2+) to create an immutable value object,
    # mirroring how hardware outputs are fixed once the gate settles.
    AdderResult = Data.define(:sum, :carry)

    # RippleCarryResult holds the output of an N-bit ripple-carry adder.
    #
    # - bits:  an Array of N result bits (LSB first, index 0 = least significant)
    # - carry: the final carry-out (1 means the result overflowed N bits)
    RippleCarryResult = Data.define(:bits, :carry)

    # -----------------------------------------------------------------------
    # Half Adder
    # -----------------------------------------------------------------------

    # Adds two single bits and returns an AdderResult.
    #
    # A half adder is the simplest addition circuit. It takes two input bits
    # and produces:
    # - sum:   XOR(a, b) — 1 if the bits are different, 0 if the same
    # - carry: AND(a, b) — 1 only if both bits are 1
    #
    # Truth table:
    #   a  b  | sum  carry
    #   0  0  |  0     0     (0 + 0 = 0)
    #   0  1  |  1     0     (0 + 1 = 1)
    #   1  0  |  1     0     (1 + 0 = 1)
    #   1  1  |  0     1     (1 + 1 = 10 in binary: sum=0, carry=1)
    #
    # It is called a "half" adder because it cannot accept a carry-in from a
    # previous stage. For multi-bit addition, we need a full adder.
    #
    # @param a [Integer] first input bit (0 or 1)
    # @param b [Integer] second input bit (0 or 1)
    # @return [AdderResult] the sum and carry bits
    def self.half_adder(a, b)
      sum_bit = LogicGates.xor_gate(a, b)
      carry = LogicGates.and_gate(a, b)
      AdderResult.new(sum: sum_bit, carry: carry)
    end

    # -----------------------------------------------------------------------
    # Full Adder
    # -----------------------------------------------------------------------

    # Adds two bits plus a carry-in and returns an AdderResult.
    #
    # A full adder extends the half adder to accept a carry-in. This is
    # essential for multi-bit addition: the carry from column N feeds into
    # column N+1.
    #
    # The circuit is built from two half adders and an OR gate:
    #
    #   Step 1: Half-add a and b → partial_sum, partial_carry
    #   Step 2: Half-add partial_sum and carry_in → final_sum, carry2
    #   Step 3: carry_out = OR(partial_carry, carry2)
    #
    # Why OR for the final carry? At most one of the two half adders can
    # produce a carry (you can't get carries from both when the inputs are
    # single bits), so OR correctly combines them.
    #
    # Truth table:
    #   a  b  cin | sum  cout
    #   0  0   0  |  0    0
    #   0  0   1  |  1    0
    #   0  1   0  |  1    0
    #   0  1   1  |  0    1
    #   1  0   0  |  1    0
    #   1  0   1  |  0    1
    #   1  1   0  |  0    1
    #   1  1   1  |  1    1
    #
    # @param a [Integer] first input bit (0 or 1)
    # @param b [Integer] second input bit (0 or 1)
    # @param carry_in [Integer] carry-in bit from previous stage (0 or 1)
    # @return [AdderResult] the sum and carry-out bits
    def self.full_adder(a, b, carry_in)
      first = half_adder(a, b)
      second = half_adder(first.sum, carry_in)
      carry_out = LogicGates.or_gate(first.carry, second.carry)
      AdderResult.new(sum: second.sum, carry: carry_out)
    end

    # -----------------------------------------------------------------------
    # Ripple-Carry Adder
    # -----------------------------------------------------------------------

    # Adds two N-bit numbers using a chain of full adders.
    #
    # This is how multi-bit addition works in hardware. Each bit position gets
    # its own full adder, and the carry output of each adder "ripples" into the
    # carry input of the next adder (from least significant to most significant).
    #
    # The name "ripple carry" describes how the carry propagates through the
    # chain like a wave. In real hardware, this ripple delay is the main
    # performance bottleneck — faster designs (carry-lookahead, carry-select)
    # exist, but ripple-carry is the simplest to understand.
    #
    # Both input arrays must be the same length and use LSB-first ordering
    # (index 0 is the least significant bit).
    #
    # @param a [Array<Integer>] first number as LSB-first bit array
    # @param b [Array<Integer>] second number as LSB-first bit array
    # @param carry_in [Integer] initial carry (default 0)
    # @return [RippleCarryResult] the sum bits (LSB-first) and final carry-out
    # @raise [ArgumentError] if a and b have different lengths or are empty
    #
    # @example Adding 5 + 3 in 4-bit
    #   a = [1, 0, 1, 0]  # 5 in LSB-first binary
    #   b = [1, 1, 0, 0]  # 3 in LSB-first binary
    #   result = CodingAdventures::Arithmetic.ripple_carry_adder(a, b)
    #   result.bits  # => [0, 0, 0, 1]  (8 in LSB-first binary)
    #   result.carry # => 0
    def self.ripple_carry_adder(a, b, carry_in: 0)
      if a.length != b.length
        raise ArgumentError,
          "a and b must have the same length, got #{a.length} and #{b.length}"
      end
      if a.empty?
        raise ArgumentError, "bit lists must not be empty"
      end

      sum_bits = []
      carry = carry_in

      a.length.times do |i|
        result = full_adder(a[i], b[i], carry)
        sum_bits << result.sum
        carry = result.carry
      end

      RippleCarryResult.new(bits: sum_bits, carry: carry)
    end
  end
end
