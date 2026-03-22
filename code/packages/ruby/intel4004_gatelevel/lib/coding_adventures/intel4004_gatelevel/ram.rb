# frozen_string_literal: true

# ---------------------------------------------------------------------------
# RAM -- 4 banks x 4 registers x 20 nibbles, built from flip-flops.
# ---------------------------------------------------------------------------
#
# === The 4004's RAM architecture ===
#
# The Intel 4004 used separate RAM chips (Intel 4002), each containing:
#     - 4 registers
#     - Each register has 16 main characters + 4 status characters
#     - Each character is a 4-bit nibble
#     - Total per chip: 4 x 20 x 4 = 320 bits
#
# The full system supports up to 4 RAM banks (4 chips), selected by the
# DCL instruction. Within a bank, the SRC instruction sets which register
# and character to access.
#
# In real hardware, each nibble is stored in 4 D flip-flops. The full
# RAM system uses 4 x 4 x 20 x 4 = 1,280 flip-flops. We simulate this
# using the register() function from the logic_gates package.
#
# === Addressing ===
#
# RAM is addressed in two steps:
#     1. DCL sets the bank (0-3, from accumulator bits 0-2)
#     2. SRC sends an 8-bit address from a register pair:
#        - High nibble -> register index (0-3)
#        - Low nibble -> character index (0-15)
# ---------------------------------------------------------------------------

require "coding_adventures_logic_gates"

module CodingAdventures
  module Intel4004Gatelevel
    class RAM
      # 4004 RAM: 4 banks x 4 registers x (16 main + 4 status) nibbles.
      #
      # Every nibble is stored in 4 D flip-flops from the sequential logic
      # package. Reading and writing physically route through flip-flop
      # state transitions.

      def initialize
        # main[bank][reg][char] = flip-flop state for one nibble
        @main = Array.new(4) do
          Array.new(4) do
            Array.new(16) do
              result = LogicGates::Sequential.register(data: [0, 0, 0, 0], clock: 0, state: nil)
              LogicGates::Sequential.register(data: [0, 0, 0, 0], clock: 1, state: result)
            end
          end
        end

        @status = Array.new(4) do
          Array.new(4) do
            Array.new(4) do
              result = LogicGates::Sequential.register(data: [0, 0, 0, 0], clock: 0, state: nil)
              LogicGates::Sequential.register(data: [0, 0, 0, 0], clock: 1, state: result)
            end
          end
        end

        # Output ports (one per bank, written by WMP)
        @output = [0, 0, 0, 0]
      end

      # Read a main character (4-bit nibble) from RAM.
      def read_main(bank, reg, char)
        state = @main[bank & 3][reg & 3][char & 0xF]
        result = LogicGates::Sequential.register(data: [0, 0, 0, 0], clock: 0, state: state)
        Bits.bits_to_int(result[:bits])
      end

      # Write a 4-bit value to a main character.
      def write_main(bank, reg, char, value)
        bits = Bits.int_to_bits(value & 0xF, 4)
        state = @main[bank & 3][reg & 3][char & 0xF]
        state = LogicGates::Sequential.register(data: bits, clock: 0, state: state)
        @main[bank & 3][reg & 3][char & 0xF] = LogicGates::Sequential.register(data: bits, clock: 1, state: state)
      end

      # Read a status character (0-3) from RAM.
      def read_status(bank, reg, index)
        state = @status[bank & 3][reg & 3][index & 3]
        result = LogicGates::Sequential.register(data: [0, 0, 0, 0], clock: 0, state: state)
        Bits.bits_to_int(result[:bits])
      end

      # Write a 4-bit value to a status character.
      def write_status(bank, reg, index, value)
        bits = Bits.int_to_bits(value & 0xF, 4)
        state = @status[bank & 3][reg & 3][index & 3]
        state = LogicGates::Sequential.register(data: bits, clock: 0, state: state)
        @status[bank & 3][reg & 3][index & 3] = LogicGates::Sequential.register(data: bits, clock: 1, state: state)
      end

      # Read a RAM output port value.
      def read_output(bank)
        @output[bank & 3]
      end

      # Write to a RAM output port (WMP instruction).
      def write_output(bank, value)
        @output[bank & 3] = value & 0xF
      end

      # Reset all RAM to 0.
      def reset
        4.times do |bank|
          4.times do |reg|
            16.times { |char| write_main(bank, reg, char, 0) }
            4.times { |stat| write_status(bank, reg, stat, 0) }
          end
          @output[bank] = 0
        end
      end

      # 4 banks x 4 regs x 20 nibbles x 4 bits x 6 gates/ff = 7680.
      # Plus addressing/decoding: ~200 gates.
      def gate_count
        7880
      end
    end
  end
end
