# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Hardware call stack -- 3 levels of 12-bit return addresses.
# ---------------------------------------------------------------------------
#
# === The 4004's stack ===
#
# The Intel 4004 has a 3-level hardware call stack. This is NOT a
# software stack in RAM -- it is three physical 12-bit registers plus
# a 2-bit circular pointer, all built from D flip-flops.
#
# Why only 3 levels? The 4004 was designed for calculators, which had
# simple call structures. Three levels of subroutine nesting was enough
# for the Busicom 141-PF calculator's firmware.
#
# === Silent overflow ===
#
# When you push a 4th address, the stack wraps silently -- the oldest
# return address is overwritten. There is no stack overflow exception.
# This matches the real hardware behavior. The 4004's designers saved
# transistors by not including overflow detection.
# ---------------------------------------------------------------------------

require "coding_adventures_logic_gates"

module CodingAdventures
  module Intel4004Gatelevel
    class HardwareStack
      # 3-level x 12-bit hardware call stack.
      #
      # Built from 3 x 12 = 36 D flip-flops for storage, plus a 2-bit
      # pointer that wraps modulo 3.

      # @return [Integer] current pointer position (not true depth, since we wrap)
      attr_reader :pointer

      def initialize
        @levels = Array.new(3) do
          result = LogicGates::Sequential.register(data: [0] * 12, clock: 0, state: nil)
          LogicGates::Sequential.register(data: [0] * 12, clock: 1, state: result)
        end
        @pointer = 0 # 0, 1, or 2
      end

      # Push a return address. Wraps silently on overflow.
      #
      # In real hardware: the pointer selects which of the 3 registers
      # to write, then the pointer increments mod 3.
      def push(address)
        bits = Bits.int_to_bits(address & 0xFFF, 12)
        state = LogicGates::Sequential.register(data: bits, clock: 0, state: @levels[@pointer])
        @levels[@pointer] = LogicGates::Sequential.register(data: bits, clock: 1, state: state)
        @pointer = (@pointer + 1) % 3
      end

      # Pop and return the top address.
      #
      # Decrements pointer mod 3, then reads that register.
      def pop
        @pointer = (@pointer - 1) % 3
        result = LogicGates::Sequential.register(data: [0] * 12, clock: 0, state: @levels[@pointer])
        Bits.bits_to_int(result[:bits])
      end

      # Reset all stack levels to 0 and pointer to 0.
      def reset
        3.times do |i|
          bits = [0] * 12
          state = LogicGates::Sequential.register(data: bits, clock: 0, state: nil)
          @levels[i] = LogicGates::Sequential.register(data: bits, clock: 1, state: state)
        end
        @pointer = 0
      end

      # Read stack level values (for inspection only).
      def read_levels
        @levels.map do |level|
          result = LogicGates::Sequential.register(data: [0] * 12, clock: 0, state: level)
          Bits.bits_to_int(result[:bits])
        end
      end

      # 3 x 12-bit registers (216 gates) + pointer logic (~10 gates).
      def gate_count
        226
      end
    end
  end
end
