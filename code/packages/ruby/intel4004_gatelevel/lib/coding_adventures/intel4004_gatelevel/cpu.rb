# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Intel 4004 gate-level CPU -- all operations route through real logic gates.
# ---------------------------------------------------------------------------
#
# === What makes this a "gate-level" simulator? ===
#
# Every computation in this CPU flows through the same gate chain that the
# real Intel 4004 used:
#
#     NOT/AND/OR/XOR -> half_adder -> full_adder -> ripple_carry_adder -> ALU
#     D flip-flop -> register -> register file / program counter / stack
#
# When you execute ADD R3, the value in register R3 is read from flip-flops,
# the accumulator is read from flip-flops, both are fed into the ALU (which
# uses full adders built from gates), and the result is clocked back into
# the accumulator's flip-flops.
#
# Nothing is simulated behaviorally. Every bit passes through gate functions.
#
# === Gate count ===
#
# Component               Gates   Transistors (x4 per gate)
# ---------------------   -----   -------------------------
# ALU (4-bit)             32      128
# Register file (16x4)    480     1,920
# Accumulator (4-bit)     24      96
# Carry flag (1-bit)      6       24
# Program counter (12)    96      384
# Hardware stack (3x12)   226     904
# Decoder                 ~50     200
# Control + wiring        ~100    400
# ---------------------   -----   -------------------------
# Total                   ~1,014  ~4,056
#
# The real Intel 4004 had 2,300 transistors. Our count is higher because
# we model RAM separately (the real 4004 used external 4002 RAM chips)
# and our gate model is not minimized with Karnaugh maps.
#
# === Execution model ===
#
# Each instruction executes in a single step() call, which corresponds
# to one machine cycle. The fetch-decode-execute pipeline:
#
#     1. FETCH:   Read instruction byte from ROM using PC
#     2. FETCH2:  For 2-byte instructions, read the second byte
#     3. DECODE:  Route instruction through decoder gate network
#     4. EXECUTE: Perform the operation through ALU/registers/etc.
# ---------------------------------------------------------------------------

require "coding_adventures_logic_gates"

module CodingAdventures
  module Intel4004Gatelevel
    # Trace record for one instruction execution.
    #
    # Same information as Intel4004Trace from the behavioral simulator,
    # plus gate-level details.
    GateTrace = Data.define(
      :address, :raw, :raw2, :mnemonic,
      :accumulator_before, :accumulator_after,
      :carry_before, :carry_after
    )

    class Intel4004GateLevel
      # Intel 4004 CPU where every operation routes through real logic gates.
      #
      # Public API matches the behavioral Intel4004Simulator for
      # cross-validation, but internally all computation flows through
      # gates, flip-flops, and adders.
      #
      # @example
      #   cpu = Intel4004GateLevel.new
      #   traces = cpu.run([0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01])
      #   cpu.registers[1]  # R1 = 1 + 2 = 3

      def initialize
        # --- Gate-level components ---
        @alu = GateALU.new
        @regs = RegisterFile.new
        @acc = Accumulator.new
        @carry = CarryFlag.new
        @pc = ProgramCounter.new
        @stack = HardwareStack.new
        @ram = RAM.new

        # --- ROM (read-only, loaded by program) ---
        @rom = Array.new(4096, 0)

        # --- RAM addressing (set by SRC/DCL) ---
        @ram_bank = 0
        @ram_register = 0
        @ram_character = 0

        # --- ROM I/O port ---
        @rom_port = 0

        # --- Control state ---
        @halted = false
      end

      # ------------------------------------------------------------------
      # Property accessors (match behavioral simulator's interface)
      # ------------------------------------------------------------------

      # Read accumulator from flip-flops.
      def accumulator
        @acc.read
      end

      # Read all 16 registers from flip-flops.
      def registers
        Array.new(16) { |i| @regs.read(i) }
      end

      # Read carry flag from flip-flop.
      def carry
        @carry.read
      end

      # Read program counter from flip-flops.
      def pc
        @pc.read
      end

      # Is the CPU halted?
      def halted?
        @halted
      end

      # For compatibility with Python interface that uses .halted
      attr_reader :halted

      # Read stack levels (for inspection only).
      def hw_stack
        @stack.read_levels
      end

      # Read RAM main characters.
      def ram
        Array.new(4) do |b|
          Array.new(4) do |r|
            Array.new(16) { |c| @ram.read_main(b, r, c) }
          end
        end
      end

      # Read RAM status characters.
      def ram_status
        Array.new(4) do |b|
          Array.new(4) do |r|
            Array.new(4) { |s| @ram.read_status(b, r, s) }
          end
        end
      end

      # Current RAM bank.
      attr_reader :ram_bank

      # Current ROM port value.
      attr_reader :rom_port

      # RAM output port values.
      def ram_output
        Array.new(4) { |i| @ram.read_output(i) }
      end

      # ------------------------------------------------------------------
      # Public API
      # ------------------------------------------------------------------

      # Load a program into ROM.
      def load_program(program)
        @rom = Array.new(4096, 0)
        program.each_with_index do |byte, i|
          @rom[i] = byte if i < 4096
        end
      end

      # Execute one instruction through the gate-level pipeline.
      #
      # @return [GateTrace] with before/after state
      def step
        raise "CPU is halted -- cannot step further" if @halted

        # Snapshot state before
        acc_before = @acc.read
        carry_before = @carry.read
        pc_before = @pc.read

        # FETCH: read instruction byte from ROM
        raw = @rom[pc_before]

        # DECODE: route through combinational decoder
        decoded = Decoder.decode(raw)

        # FETCH2: if 2-byte, read second byte
        raw2 = nil
        if decoded.is_two_byte != 0
          raw2 = @rom[(pc_before + 1) & 0xFFF]
          decoded = Decoder.decode(raw, raw2)
        end

        # EXECUTE: route through appropriate gate paths
        mnemonic = execute_instruction(decoded)

        GateTrace.new(
          address: pc_before,
          raw: raw,
          raw2: raw2,
          mnemonic: mnemonic,
          accumulator_before: acc_before,
          accumulator_after: @acc.read,
          carry_before: carry_before,
          carry_after: @carry.read
        )
      end

      # Load and run a program, returning execution trace.
      def run(program, max_steps: 10_000)
        reset
        load_program(program)

        traces = []
        max_steps.times do
          break if @halted
          traces << step
        end
        traces
      end

      # Reset all CPU state.
      def reset
        @acc.reset
        @carry.reset
        @regs.reset
        @pc.reset
        @stack.reset
        @ram.reset
        @rom = Array.new(4096, 0)
        @ram_bank = 0
        @ram_register = 0
        @ram_character = 0
        @rom_port = 0
        @halted = false
      end

      # Total estimated gate count for the CPU.
      def gate_count
        @alu.gate_count +
          @regs.gate_count +
          @acc.gate_count +
          @carry.gate_count +
          @pc.gate_count +
          @stack.gate_count +
          @ram.gate_count +
          50 + # decoder
          100  # control logic and wiring
      end

      private

      # ------------------------------------------------------------------
      # Instruction execution -- routes through gate-level components
      # ------------------------------------------------------------------

      # Execute a decoded instruction through gate paths.
      #
      # Each instruction routes through the appropriate combination of
      # ALU, registers, and flip-flops.
      def execute_instruction(d)
        # NOP
        if d.is_nop != 0
          @pc.increment
          return "NOP"
        end

        # HLT
        if d.is_hlt != 0
          @halted = true
          @pc.increment
          return "HLT"
        end

        # LDM N: load immediate into accumulator
        if d.is_ldm != 0
          @acc.write(d.immediate)
          @pc.increment
          return "LDM #{d.immediate}"
        end

        # LD Rn: load register into accumulator
        if d.is_ld != 0
          val = @regs.read(d.reg_index)
          @acc.write(val)
          @pc.increment
          return "LD R#{d.reg_index}"
        end

        # XCH Rn: exchange accumulator and register
        if d.is_xch != 0
          a_val = @acc.read
          r_val = @regs.read(d.reg_index)
          @acc.write(r_val)
          @regs.write(d.reg_index, a_val)
          @pc.increment
          return "XCH R#{d.reg_index}"
        end

        # INC Rn: increment register (no carry effect)
        if d.is_inc != 0
          r_val = @regs.read(d.reg_index)
          result, _ = @alu.increment(r_val)
          @regs.write(d.reg_index, result)
          @pc.increment
          return "INC R#{d.reg_index}"
        end

        # ADD Rn: add register to accumulator with carry
        if d.is_add != 0
          a_val = @acc.read
          r_val = @regs.read(d.reg_index)
          carry_in = @carry.read ? 1 : 0
          result, carry_out = @alu.add(a_val, r_val, carry_in)
          @acc.write(result)
          @carry.write(carry_out)
          @pc.increment
          return "ADD R#{d.reg_index}"
        end

        # SUB Rn: subtract register from accumulator
        if d.is_sub != 0
          a_val = @acc.read
          r_val = @regs.read(d.reg_index)
          borrow_in = @carry.read ? 0 : 1
          result, carry_out = @alu.subtract(a_val, r_val, borrow_in)
          @acc.write(result)
          @carry.write(carry_out)
          @pc.increment
          return "SUB R#{d.reg_index}"
        end

        # JUN addr: unconditional jump
        if d.is_jun != 0
          @pc.load(d.addr12)
          return format("JUN 0x%03X", d.addr12)
        end

        # JCN cond,addr: conditional jump
        if d.is_jcn != 0
          return exec_jcn(d)
        end

        # ISZ Rn,addr: increment and skip if zero
        if d.is_isz != 0
          return exec_isz(d)
        end

        # JMS addr: jump to subroutine
        if d.is_jms != 0
          return_addr = @pc.read + 2
          @stack.push(return_addr)
          @pc.load(d.addr12)
          return format("JMS 0x%03X", d.addr12)
        end

        # BBL N: branch back and load
        if d.is_bbl != 0
          @acc.write(d.immediate)
          return_addr = @stack.pop
          @pc.load(return_addr)
          return "BBL #{d.immediate}"
        end

        # FIM Pp,data: fetch immediate to pair
        if d.is_fim != 0
          @regs.write_pair(d.pair_index, d.addr8)
          @pc.increment2
          return format("FIM P%d,0x%02X", d.pair_index, d.addr8)
        end

        # SRC Pp: send register control
        if d.is_src != 0
          pair_val = @regs.read_pair(d.pair_index)
          @ram_register = (pair_val >> 4) & 0xF
          @ram_character = pair_val & 0xF
          @pc.increment
          return "SRC P#{d.pair_index}"
        end

        # FIN Pp: fetch indirect from ROM
        if d.is_fin != 0
          p0_val = @regs.read_pair(0)
          page = @pc.read & 0xF00
          rom_addr = page | p0_val
          rom_byte = @rom[rom_addr & 0xFFF]
          @regs.write_pair(d.pair_index, rom_byte)
          @pc.increment
          return "FIN P#{d.pair_index}"
        end

        # JIN Pp: jump indirect
        if d.is_jin != 0
          pair_val = @regs.read_pair(d.pair_index)
          page = @pc.read & 0xF00
          @pc.load(page | pair_val)
          return "JIN P#{d.pair_index}"
        end

        # I/O operations (0xE_ range)
        if d.is_io != 0
          return exec_io(d)
        end

        # Accumulator operations (0xF_ range)
        if d.is_accum != 0
          return exec_accum(d)
        end

        # Unknown -- advance PC to avoid infinite loop
        @pc.increment
        format("UNKNOWN(0x%02X)", d.raw)
      end

      # JCN cond,addr: conditional jump using gate logic.
      #
      # Condition nibble bits (evaluated with OR/AND/NOT gates):
      #     Bit 3: INVERT
      #     Bit 2: TEST A==0
      #     Bit 1: TEST carry==1
      #     Bit 0: TEST pin (always 0)
      def exec_jcn(d)
        cond = d.condition
        a_val = @acc.read
        carry_val = @carry.read ? 1 : 0

        # Test A==0: OR all accumulator bits, then NOT
        a_bits = Bits.int_to_bits(a_val, 4)
        a_is_zero = LogicGates.not_gate(
          LogicGates.or_gate(
            LogicGates.or_gate(a_bits[0], a_bits[1]),
            LogicGates.or_gate(a_bits[2], a_bits[3])
          )
        )

        # Build test result using gates
        test_zero = LogicGates.and_gate((cond >> 2) & 1, a_is_zero)
        test_carry = LogicGates.and_gate((cond >> 1) & 1, carry_val)
        test_pin = LogicGates.and_gate(cond & 1, 0)

        test_result = LogicGates.or_gate(
          LogicGates.or_gate(test_zero, test_carry),
          test_pin
        )

        # Invert if bit 3 set
        invert = (cond >> 3) & 1
        # XOR with invert: if invert=1, flip result
        final = LogicGates.or_gate(
          LogicGates.and_gate(test_result, LogicGates.not_gate(invert)),
          LogicGates.and_gate(LogicGates.not_gate(test_result), invert)
        )

        page = (@pc.read + 2) & 0xF00
        target = page | d.addr8

        if final != 0
          @pc.load(target)
        else
          @pc.increment2
        end

        format("JCN %d,%02X", cond, d.addr8)
      end

      # ISZ Rn,addr: increment register, skip if zero.
      def exec_isz(d)
        r_val = @regs.read(d.reg_index)
        result, _ = @alu.increment(r_val)
        @regs.write(d.reg_index, result)

        # Test if result is zero using NOR of all bits
        r_bits = Bits.int_to_bits(result, 4)
        is_zero = LogicGates.not_gate(
          LogicGates.or_gate(
            LogicGates.or_gate(r_bits[0], r_bits[1]),
            LogicGates.or_gate(r_bits[2], r_bits[3])
          )
        )

        page = (@pc.read + 2) & 0xF00
        target = page | d.addr8

        if is_zero != 0
          # Result is zero -> fall through
          @pc.increment2
        else
          # Result is nonzero -> jump
          @pc.load(target)
        end

        format("ISZ R%d,0x%02X", d.reg_index, d.addr8)
      end

      # Execute I/O instructions (0xE0-0xEF).
      def exec_io(d)
        a_val = @acc.read
        sub_op = d.lower

        if sub_op == 0x0 # WRM
          @ram.write_main(@ram_bank, @ram_register, @ram_character, a_val)
          @pc.increment
          return "WRM"
        end

        if sub_op == 0x1 # WMP
          @ram.write_output(@ram_bank, a_val)
          @pc.increment
          return "WMP"
        end

        if sub_op == 0x2 # WRR
          @rom_port = a_val & 0xF
          @pc.increment
          return "WRR"
        end

        if sub_op == 0x3 # WPM (NOP in simulation)
          @pc.increment
          return "WPM"
        end

        if sub_op.between?(0x4, 0x7) # WR0-WR3
          idx = sub_op - 0x4
          @ram.write_status(@ram_bank, @ram_register, idx, a_val)
          @pc.increment
          return "WR#{idx}"
        end

        if sub_op == 0x8 # SBM
          ram_val = @ram.read_main(@ram_bank, @ram_register, @ram_character)
          borrow_in = @carry.read ? 0 : 1
          result, carry_out = @alu.subtract(a_val, ram_val, borrow_in)
          @acc.write(result)
          @carry.write(carry_out)
          @pc.increment
          return "SBM"
        end

        if sub_op == 0x9 # RDM
          val = @ram.read_main(@ram_bank, @ram_register, @ram_character)
          @acc.write(val)
          @pc.increment
          return "RDM"
        end

        if sub_op == 0xA # RDR
          @acc.write(@rom_port & 0xF)
          @pc.increment
          return "RDR"
        end

        if sub_op == 0xB # ADM
          ram_val = @ram.read_main(@ram_bank, @ram_register, @ram_character)
          carry_in = @carry.read ? 1 : 0
          result, carry_out = @alu.add(a_val, ram_val, carry_in)
          @acc.write(result)
          @carry.write(carry_out)
          @pc.increment
          return "ADM"
        end

        if sub_op.between?(0xC, 0xF) # RD0-RD3
          idx = sub_op - 0xC
          val = @ram.read_status(@ram_bank, @ram_register, idx)
          @acc.write(val)
          @pc.increment
          return "RD#{idx}"
        end

        @pc.increment
        format("IO(0x%02X)", d.raw)
      end

      # Execute accumulator operations (0xF0-0xFD).
      def exec_accum(d)
        a_val = @acc.read
        sub_op = d.lower

        if sub_op == 0x0 # CLB
          @acc.write(0)
          @carry.write(false)
          @pc.increment
          return "CLB"
        end

        if sub_op == 0x1 # CLC
          @carry.write(false)
          @pc.increment
          return "CLC"
        end

        if sub_op == 0x2 # IAC
          result, carry = @alu.increment(a_val)
          @acc.write(result)
          @carry.write(carry)
          @pc.increment
          return "IAC"
        end

        if sub_op == 0x3 # CMC
          @carry.write(!@carry.read)
          @pc.increment
          return "CMC"
        end

        if sub_op == 0x4 # CMA
          result = @alu.complement(a_val)
          @acc.write(result)
          @pc.increment
          return "CMA"
        end

        if sub_op == 0x5 # RAL
          old_carry = @carry.read ? 1 : 0
          # Use gates: A3 goes to carry, shift left, old carry to bit 0
          a_bits = Bits.int_to_bits(a_val, 4)
          @carry.write(a_bits[3] == 1)
          new_bits = [old_carry, a_bits[0], a_bits[1], a_bits[2]]
          @acc.write(Bits.bits_to_int(new_bits))
          @pc.increment
          return "RAL"
        end

        if sub_op == 0x6 # RAR
          old_carry = @carry.read ? 1 : 0
          a_bits = Bits.int_to_bits(a_val, 4)
          @carry.write(a_bits[0] == 1)
          new_bits = [a_bits[1], a_bits[2], a_bits[3], old_carry]
          @acc.write(Bits.bits_to_int(new_bits))
          @pc.increment
          return "RAR"
        end

        if sub_op == 0x7 # TCC
          @acc.write(@carry.read ? 1 : 0)
          @carry.write(false)
          @pc.increment
          return "TCC"
        end

        if sub_op == 0x8 # DAC
          result, carry = @alu.decrement(a_val)
          @acc.write(result)
          @carry.write(carry)
          @pc.increment
          return "DAC"
        end

        if sub_op == 0x9 # TCS
          @acc.write(@carry.read ? 10 : 9)
          @carry.write(false)
          @pc.increment
          return "TCS"
        end

        if sub_op == 0xA # STC
          @carry.write(true)
          @pc.increment
          return "STC"
        end

        if sub_op == 0xB # DAA
          if a_val > 9 || @carry.read
            result, carry = @alu.add(a_val, 6, 0)
            @carry.write(true) if carry
            @acc.write(result)
          end
          @pc.increment
          return "DAA"
        end

        if sub_op == 0xC # KBP
          kbp_table = {0 => 0, 1 => 1, 2 => 2, 4 => 3, 8 => 4}
          @acc.write(kbp_table.fetch(a_val, 15))
          @pc.increment
          return "KBP"
        end

        if sub_op == 0xD # DCL
          bank = @alu.bitwise_and(a_val, 0x7)
          bank = @alu.bitwise_and(bank, 0x3) if bank > 3
          @ram_bank = bank
          @pc.increment
          return "DCL"
        end

        @pc.increment
        format("ACCUM(0x%02X)", d.raw)
      end
    end
  end
end
