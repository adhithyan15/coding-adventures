# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Bit conversion helpers -- the bridge between integers and gate-level bits.
# ---------------------------------------------------------------------------
#
# === Why this module exists ===
#
# The gate-level simulator operates on individual bits (arrays of 0s and 1s),
# because that is what real hardware does. But the outside world (test programs,
# the behavioral simulator) works with integers. This module converts between
# the two representations.
#
# === Bit ordering: LSB first ===
#
# All bit arrays use LSB-first ordering, matching the logic-gates and arithmetic
# packages. Index 0 is the least significant bit.
#
#     int_to_bits(5, 4)  =>  [1, 0, 1, 0]
#     #                       bit0=1(x1) + bit1=0(x2) + bit2=1(x4) + bit3=0(x8) = 5
#
# This convention is used throughout the computing stack because it maps
# naturally to how adders chain: bit 0 feeds the first full adder, bit 1
# feeds the second, and so on.
# ---------------------------------------------------------------------------

module CodingAdventures
  module Intel4004Gatelevel
    module Bits
      # Convert an integer to an array of bits (LSB first).
      #
      # @param value [Integer] non-negative integer to convert
      # @param width [Integer] number of bits in the output array
      # @return [Array<Integer>] array of 0s and 1s, length = width, LSB at index 0
      #
      # @example
      #   Bits.int_to_bits(5, 4)  # => [1, 0, 1, 0]
      #   Bits.int_to_bits(0, 4)  # => [0, 0, 0, 0]
      #   Bits.int_to_bits(15, 4) # => [1, 1, 1, 1]
      def self.int_to_bits(value, width)
        # Mask to width to handle negative or oversized values
        value &= ((1 << width) - 1)
        Array.new(width) { |i| (value >> i) & 1 }
      end

      # Convert an array of bits (LSB first) to an integer.
      #
      # @param bits [Array<Integer>] array of 0s and 1s, LSB at index 0
      # @return [Integer] non-negative integer
      #
      # @example
      #   Bits.bits_to_int([1, 0, 1, 0])  # => 5
      #   Bits.bits_to_int([0, 0, 0, 0])  # => 0
      #   Bits.bits_to_int([1, 1, 1, 1])  # => 15
      def self.bits_to_int(bits)
        result = 0
        bits.each_with_index do |bit, i|
          result |= bit << i
        end
        result
      end
    end
  end
end
