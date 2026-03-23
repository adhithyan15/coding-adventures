# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Program counter -- 12-bit register with increment and load.
# ---------------------------------------------------------------------------
#
# === The 4004's program counter ===
#
# The program counter (PC) holds the address of the next instruction to
# fetch from ROM. It is 12 bits wide, addressing 4096 bytes of ROM.
#
# In real hardware, the PC is:
# - A 12-bit register (12 D flip-flops)
# - An incrementer (chain of half-adders for PC+1 or PC+2)
# - A load input (for jump instructions)
#
# The incrementer uses half-adders chained together. To add 1:
#     bit0 -> half_adder(bit0, 1) -> sum0, carry
#     bit1 -> half_adder(bit1, carry) -> sum1, carry
#     ...and so on for all 12 bits.
#
# This is simpler than a full adder chain because we are always adding
# a constant (1 or 2), so one input is fixed.
# ---------------------------------------------------------------------------

require "coding_adventures_logic_gates"
require "coding_adventures_arithmetic"

module CodingAdventures
  module Intel4004Gatelevel
    class ProgramCounter
      # 12-bit program counter built from flip-flops and half-adders.
      #
      # Supports:
      #     - increment: PC += 1 (for 1-byte instructions)
      #     - increment2: PC += 2 (for 2-byte instructions)
      #     - load(addr): PC = addr (for jumps)
      #     - read: current PC value

      def initialize
        result = LogicGates::Sequential.register(data: [0] * 12, clock: 0, state: nil)
        @state = LogicGates::Sequential.register(data: [0] * 12, clock: 1, state: result)
      end

      # Read current PC value (0-4095).
      def read
        result = LogicGates::Sequential.register(data: [0] * 12, clock: 0, state: @state)
        Bits.bits_to_int(result[:bits])
      end

      # Load a new address into the PC (for jumps).
      def load(address)
        bits = Bits.int_to_bits(address & 0xFFF, 12)
        state = LogicGates::Sequential.register(data: bits, clock: 0, state: @state)
        @state = LogicGates::Sequential.register(data: bits, clock: 1, state: state)
      end

      # Increment PC by 1 using a chain of half-adders.
      #
      # This is how a real incrementer works:
      #     carry_in = 1 (we are adding 1)
      #     For each bit position:
      #         (new_bit, carry) = half_adder(old_bit, carry)
      def increment
        current_bits = Bits.int_to_bits(read, 12)
        carry = 1 # Adding 1
        new_bits = []
        current_bits.each do |bit|
          result = Arithmetic.half_adder(bit, carry)
          new_bits << result.sum
          carry = result.carry
        end
        load(Bits.bits_to_int(new_bits))
      end

      # Increment PC by 2 (for 2-byte instructions).
      #
      # Two cascaded increments through the half-adder chain.
      def increment2
        increment
        increment
      end

      # Reset PC to 0.
      def reset
        load(0)
      end

      # 12-bit register (72 gates) + 12 half-adders (24 gates) = 96.
      def gate_count
        96
      end
    end
  end
end
