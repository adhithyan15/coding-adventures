# frozen_string_literal: true

# === Intel 4004 Simulator — the world's first commercial microprocessor ===
#
# The Intel 4004 was released in 1971, designed by Federico Faggin, Ted Hoff,
# and Stanley Mazor for the Busicom 141-PF calculator. It contained just 2,300
# transistors and ran at 740 kHz — about a million times slower than modern CPUs.
# Yet it proved a general-purpose processor could be built on a single chip.
#
# === Why 4-bit? ===
#
# Every data value is 4 bits wide (0-15). This was perfect for calculators,
# which use Binary-Coded Decimal (BCD) — a single decimal digit (0-9) fits
# in 4 bits. All values are masked to 4 bits (& 0xF).
#
# === Accumulator architecture ===
#
# Almost every arithmetic operation works through the Accumulator (A):
#   - To add: load into A, store in register, load other, ADD register
#   - No "add register to register" instruction
#
# Compare with other architectures:
#   RISC-V (register-register):  add x3, x1, x2
#   WASM (stack-based):          i32.add
#   4004 (accumulator):          ADD R0   (A = A + R0)
#
# === Instruction encoding ===
#
# Instructions are 8 bits. Upper nibble = opcode, lower nibble = operand.
#
#   LDM N   (0xDN): Load immediate N into accumulator
#   XCH RN  (0xBN): Exchange accumulator with register N
#   ADD RN  (0x8N): Add register N to accumulator
#   SUB RN  (0x9N): Subtract register N from accumulator
#   HLT     (0x01): Halt (simulator-only, not real 4004)

module CodingAdventures
  module Intel4004Simulator
    # Trace of a single instruction execution — immutable
    Intel4004Trace = Data.define(:address, :raw, :mnemonic,
      :accumulator_before, :accumulator_after,
      :carry_before, :carry_after)

    # -------------------------------------------------------------------
    # Simulator
    # -------------------------------------------------------------------
    class Intel4004Sim
      attr_reader :accumulator, :registers, :carry, :halted
      attr_accessor :pc

      def initialize(memory_size: 4096)
        @accumulator = 0
        @registers = Array.new(16, 0)
        @carry = false
        @memory = Array.new(memory_size, 0)
        @pc = 0
        @halted = false
      end

      def load_program(program)
        program.each_byte.with_index do |byte, i|
          @memory[i] = byte
        end
      end

      # Fetch, decode, and execute one instruction.
      def step
        raise RuntimeError, "CPU is halted — cannot step further" if @halted

        address = @pc
        raw = @memory[@pc]
        @pc += 1

        acc_before = @accumulator
        carry_before = @carry

        opcode = (raw >> 4) & 0xF
        operand = raw & 0xF

        mnemonic = execute_instruction(opcode, operand, raw)

        Intel4004Trace.new(
          address: address, raw: raw, mnemonic: mnemonic,
          accumulator_before: acc_before, accumulator_after: @accumulator,
          carry_before: carry_before, carry_after: @carry
        )
      end

      # Load and run a program, returning traces.
      def run(program, max_steps: 10_000)
        load_program(program)
        traces = []
        max_steps.times do
          trace = step
          traces << trace
          break if @halted
        end
        traces
      end

      private

      def execute_instruction(opcode, operand, raw)
        case opcode
        when 0xD
          # LDM N: Load immediate into accumulator
          @accumulator = operand & 0xF
          "LDM #{operand}"
        when 0xB
          # XCH RN: Exchange accumulator with register
          reg = operand & 0xF
          old_a = @accumulator
          @accumulator = @registers[reg] & 0xF
          @registers[reg] = old_a & 0xF
          "XCH R#{reg}"
        when 0x8
          # ADD RN: Add register to accumulator
          reg = operand & 0xF
          result = @accumulator + @registers[reg]
          @carry = result > 0xF
          @accumulator = result & 0xF
          "ADD R#{reg}"
        when 0x9
          # SUB RN: Subtract register from accumulator
          reg = operand & 0xF
          result = @accumulator - @registers[reg]
          @carry = result < 0
          @accumulator = result & 0xF
          "SUB R#{reg}"
        else
          if raw == 0x01
            # HLT: Halt execution
            @halted = true
            "HLT"
          else
            "UNKNOWN(0x#{format("%02X", raw)})"
          end
        end
      end
    end
  end
end
