# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Register file -- 16 x 4-bit registers built from D flip-flops.
# ---------------------------------------------------------------------------
#
# === How registers work in hardware ===
#
# A register is a group of D flip-flops that share a clock signal. Each
# flip-flop stores one bit. A 4-bit register has 4 flip-flops. The Intel
# 4004 has 16 such registers (R0-R15), for a total of 64 flip-flops just
# for the register file.
#
# In this simulation, each register call goes through:
#     data bits -> D flip-flop x 4 -> output bits
#
# The flip-flops are edge-triggered: they capture new data on the rising
# edge of the clock. Between edges, the stored value is stable.
#
# === Register pairs ===
#
# The 4004 organizes its 16 registers into 8 pairs:
#     P0 = R0:R1, P1 = R2:R3, ..., P7 = R14:R15
#
# A register pair holds an 8-bit value (high nibble in even register,
# low nibble in odd register). Pairs are used for:
#     - FIM: load 8-bit immediate
#     - SRC: set RAM address
#     - FIN: indirect ROM read
#     - JIN: indirect jump
#
# === Accumulator ===
#
# The accumulator is a separate 4-bit register, not part of the R0-R15
# file. It has its own dedicated flip-flops and is connected directly to
# the ALU's output bus.
# ---------------------------------------------------------------------------

require "coding_adventures_logic_gates"

module CodingAdventures
  module Intel4004Gatelevel
    class RegisterFile
      # 16 x 4-bit register file built from D flip-flops.
      #
      # Each of the 16 registers is a group of 4 D flip-flops from the
      # logic_gates sequential module. Reading and writing go through
      # actual flip-flop state transitions.

      def initialize
        # Each register's state is a hash with :ff_states key
        # Initialize all to 0 by clocking in zeros
        @states = Array.new(16) do
          result = LogicGates::Sequential.register(data: [0, 0, 0, 0], clock: 0, state: nil)
          result2 = LogicGates::Sequential.register(data: [0, 0, 0, 0], clock: 1, state: result)
          result2
        end
      end

      # Read a register value. Returns 4-bit integer (0-15).
      #
      # In real hardware, this would route through a 16-to-1 multiplexer
      # built from gates. We simulate the flip-flop read directly.
      def read(index)
        # Read current output from flip-flops (clock=0, no write)
        result = LogicGates::Sequential.register(data: [0, 0, 0, 0], clock: 0, state: @states[index])
        Bits.bits_to_int(result[:bits])
      end

      # Write a 4-bit value to a register.
      #
      # In real hardware: decoder selects the register, data bus presents
      # the value, clock edge latches it into the flip-flops.
      def write(index, value)
        bits = Bits.int_to_bits(value & 0xF, 4)
        # Clock low (setup)
        state = LogicGates::Sequential.register(data: bits, clock: 0, state: @states[index])
        # Clock high (capture on rising edge)
        @states[index] = LogicGates::Sequential.register(data: bits, clock: 1, state: state)
      end

      # Read an 8-bit value from a register pair.
      #
      # Pair 0 = R0:R1 (R0=high nibble, R1=low nibble).
      def read_pair(pair_index)
        high = read(pair_index * 2)
        low = read(pair_index * 2 + 1)
        (high << 4) | low
      end

      # Write an 8-bit value to a register pair.
      def write_pair(pair_index, value)
        write(pair_index * 2, (value >> 4) & 0xF)
        write(pair_index * 2 + 1, value & 0xF)
      end

      # Reset all registers to 0 by clocking in zeros.
      def reset
        16.times { |i| write(i, 0) }
      end

      # Gate count for the register file.
      #
      # 16 registers x 4 bits x ~6 gates per D flip-flop = 384 gates.
      # Plus 4-to-16 decoder for write select: ~32 gates.
      # Plus 16-to-1 mux for read select: ~64 gates.
      # Total: ~480 gates.
      def gate_count
        480
      end
    end

    class Accumulator
      # 4-bit accumulator register built from D flip-flops.
      #
      # The accumulator is the 4004's main working register. Almost every
      # arithmetic and logic operation reads from or writes to it.

      def initialize
        result = LogicGates::Sequential.register(data: [0, 0, 0, 0], clock: 0, state: nil)
        @state = LogicGates::Sequential.register(data: [0, 0, 0, 0], clock: 1, state: result)
      end

      # Read the accumulator value (0-15).
      def read
        result = LogicGates::Sequential.register(data: [0, 0, 0, 0], clock: 0, state: @state)
        Bits.bits_to_int(result[:bits])
      end

      # Write a 4-bit value to the accumulator.
      def write(value)
        bits = Bits.int_to_bits(value & 0xF, 4)
        state = LogicGates::Sequential.register(data: bits, clock: 0, state: @state)
        @state = LogicGates::Sequential.register(data: bits, clock: 1, state: state)
      end

      # Reset to 0.
      def reset
        write(0)
      end

      # 4 D flip-flops x ~6 gates = 24 gates.
      def gate_count
        24
      end
    end

    class CarryFlag
      # 1-bit carry/borrow flag built from a D flip-flop.
      #
      # The carry flag is set by arithmetic operations and read by
      # conditional jumps and multi-digit BCD arithmetic.

      def initialize
        result = LogicGates::Sequential.register(data: [0], clock: 0, state: nil)
        @state = LogicGates::Sequential.register(data: [0], clock: 1, state: result)
      end

      # Read carry flag as a boolean.
      def read
        result = LogicGates::Sequential.register(data: [0], clock: 0, state: @state)
        result[:bits][0] == 1
      end

      # Write carry flag.
      def write(value)
        bit = [value ? 1 : 0]
        state = LogicGates::Sequential.register(data: bit, clock: 0, state: @state)
        @state = LogicGates::Sequential.register(data: bit, clock: 1, state: state)
      end

      # Reset to false.
      def reset
        write(false)
      end

      # 1 D flip-flop x ~6 gates = 6 gates.
      def gate_count
        6
      end
    end
  end
end
