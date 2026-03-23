# frozen_string_literal: true

# === Intel 4004 Simulator -- the world's first commercial microprocessor ===
#
# The Intel 4004 was released in 1971, designed by Federico Faggin, Ted Hoff,
# and Stanley Mazor for the Busicom 141-PF calculator. It contained just 2,300
# transistors and ran at 740 kHz -- about a million times slower than modern CPUs.
# Yet it proved a general-purpose processor could be built on a single chip.
#
# === Why 4-bit? ===
#
# Every data value is 4 bits wide (0-15). This was perfect for calculators,
# which use Binary-Coded Decimal (BCD) -- a single decimal digit (0-9) fits
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
# Some instructions are 2 bytes (JCN, FIM, JUN, JMS, ISZ) -- the second
# byte provides an address or immediate data.
#
# === Complete Instruction Set (46 instructions) ===
#
#   0x00       NOP          No operation
#   0x01       HLT          Halt (simulator-only)
#   0x1_       JCN c,a  *   Conditional jump (c=condition nibble)
#   0x2_ even  FIM Pp,d *   Fetch immediate to register pair
#   0x2_ odd   SRC Pp       Send register control (pair as address)
#   0x3_ even  FIN Pp       Fetch indirect from ROM via P0
#   0x3_ odd   JIN Pp       Jump indirect via register pair
#   0x4_       JUN a    *   Unconditional jump (12-bit address)
#   0x5_       JMS a    *   Jump to subroutine
#   0x6_       INC Rn       Increment register
#   0x7_       ISZ Rn,a *   Increment and skip if zero
#   0x8_       ADD Rn       Add register to accumulator
#   0x9_       SUB Rn       Subtract register from accumulator
#   0xA_       LD Rn        Load register into accumulator
#   0xB_       XCH Rn       Exchange accumulator and register
#   0xC_       BBL n        Branch back and load
#   0xD_       LDM n        Load immediate into accumulator
#   0xE0-0xEF  I/O ops      RAM/ROM read/write operations
#   0xF0-0xFD  Accum ops    Accumulator manipulation
#
#   * = 2-byte instruction (second byte is data or address)
#
# === Memory Model ===
#
# The 4004's memory system has two distinct parts:
#
# ROM (Read-Only Memory): 4096 x 8-bit bytes for program storage.
#   - Addressed by the 12-bit PC (Program Counter).
#   - Also has I/O ports for external devices (rom_port).
#
# RAM (Random Access Memory): organized in a hierarchy:
#   - 4 banks (selected by DCL instruction)
#   - 4 registers per bank (high nibble of SRC address)
#   - 16 main characters per register (low nibble of SRC address)
#   - 4 status characters per register (accessed by RD0-RD3, WR0-WR3)
#   - 1 output port per bank (written by WMP)
#
# Total RAM: 4 x 4 x (16 + 4) = 320 nibbles = 160 bytes
#
# === Hardware Call Stack ===
#
# The 4004 has a 3-level hardware call stack for subroutine calls.
# Each level stores a 12-bit return address. The stack wraps silently
# on overflow -- the 4th push overwrites the oldest entry. There is
# no stack overflow exception. JMS pushes, BBL pops.

module CodingAdventures
  module Intel4004Simulator
    # Trace of a single instruction execution -- immutable.
    # Captures before/after snapshots for debugging and verification.
    Intel4004Trace = Data.define(:address, :raw, :raw2, :mnemonic,
      :accumulator_before, :accumulator_after,
      :carry_before, :carry_after)

    # -------------------------------------------------------------------
    # Simulator -- implements all 46 Intel 4004 instructions
    # -------------------------------------------------------------------
    class Intel4004Sim
      attr_reader :accumulator, :registers, :carry, :halted,
        :hw_stack, :stack_pointer,
        :ram, :ram_status, :ram_output,
        :ram_bank, :ram_register, :ram_character,
        :rom_port
      attr_accessor :pc

      # === Initialization ===
      #
      # Set up all CPU state to match a freshly powered-on 4004:
      #   - Accumulator and all registers zeroed
      #   - Carry flag cleared
      #   - PC at address 0
      #   - All RAM cleared
      #   - Stack empty (all zeros, pointer at 0)
      def initialize(memory_size: 4096)
        @memory_size = memory_size
        @memory = Array.new(memory_size, 0)
        reset
      end

      # Reset all CPU state to power-on defaults.
      def reset
        @accumulator = 0
        @registers = Array.new(16, 0)
        @carry = false
        @pc = 0
        @halted = false

        # --- Hardware call stack ---
        # 3 levels of 12-bit return addresses, wrapping on overflow.
        @hw_stack = [0, 0, 0]
        @stack_pointer = 0

        # --- RAM ---
        # 4 banks x 4 registers x 16 main characters (4-bit nibbles)
        @ram = Array.new(4) { Array.new(4) { Array.new(16, 0) } }
        # 4 banks x 4 registers x 4 status characters
        @ram_status = Array.new(4) { Array.new(4) { Array.new(4, 0) } }
        # RAM output port: one per bank, written by WMP
        @ram_output = Array.new(4, 0)

        # --- RAM addressing (set by SRC and DCL) ---
        @ram_bank = 0       # selected by DCL (0-7, but only 0-3 used)
        @ram_register = 0   # high nibble of SRC pair value
        @ram_character = 0  # low nibble of SRC pair value

        # --- ROM I/O port ---
        @rom_port = 0

        @memory.fill(0)
      end

      # Load a program (binary string) into ROM starting at address 0.
      def load_program(program)
        program.each_byte.with_index do |byte, i|
          @memory[i] = byte if i < @memory_size
        end
      end

      # === Fetch-Decode-Execute ===
      #
      # The heart of any CPU simulator. Each call to step:
      #   1. Fetches the byte at the current PC
      #   2. Checks if it's a 2-byte instruction (fetches second byte if so)
      #   3. Snapshots the accumulator and carry (for the trace)
      #   4. Decodes and dispatches to the appropriate handler
      #   5. Returns an immutable trace record
      def step
        raise "CPU is halted -- cannot step further" if @halted

        address = @pc
        raw = fetch_byte
        raw2 = nil

        # Some instructions are 2 bytes -- fetch the second byte now.
        # The PC has already advanced past byte 1, so fetch_byte gets byte 2.
        if two_byte_instruction?(raw)
          raw2 = fetch_byte
        end

        acc_before = @accumulator
        carry_before = @carry

        mnemonic = execute_instruction(raw, raw2, address)

        Intel4004Trace.new(
          address: address, raw: raw, raw2: raw2, mnemonic: mnemonic,
          accumulator_before: acc_before, accumulator_after: @accumulator,
          carry_before: carry_before, carry_after: @carry
        )
      end

      # Load and run a program, returning an array of traces.
      # Execution continues until HLT or max_steps is reached.
      def run(program, max_steps: 10_000)
        reset
        load_program(program)
        traces = []
        max_steps.times do
          break if @halted || @pc >= @memory_size
          trace = step
          traces << trace
          break if @halted
        end
        traces
      end

      # Predicate: is the CPU halted?
      def halted?
        @halted
      end

      private

      # Fetch one byte from ROM at the current PC and advance PC.
      def fetch_byte
        byte = @memory[@pc] || 0
        @pc = (@pc + 1) & 0xFFF  # 12-bit PC wraps at 4096
        byte
      end

      # === 2-byte instruction detection ===
      #
      # The 4004 has five 2-byte instruction families:
      #   0x1_ JCN  -- conditional jump
      #   0x2_ FIM  -- fetch immediate (even lower nibble only)
      #   0x4_ JUN  -- unconditional jump
      #   0x5_ JMS  -- jump to subroutine
      #   0x7_ ISZ  -- increment and skip if zero
      def two_byte_instruction?(raw)
        upper = (raw >> 4) & 0xF
        return true if [0x1, 0x4, 0x5, 0x7].include?(upper)
        # FIM is 0x2_ with even lower nibble
        upper == 0x2 && (raw & 0x1) == 0
      end

      # === Register Pair Helpers ===
      #
      # The 4004's 16 registers are organized as 8 pairs:
      #   Pair 0 = R0:R1, Pair 1 = R2:R3, ..., Pair 7 = R14:R15
      # The even register holds the high nibble, odd holds the low nibble.
      # Together they form an 8-bit value (0-255).

      # Read an 8-bit value from a register pair.
      def read_pair(pair_idx)
        high_reg = pair_idx * 2
        low_reg = high_reg + 1
        (@registers[high_reg] << 4) | @registers[low_reg]
      end

      # Write an 8-bit value to a register pair.
      def write_pair(pair_idx, value)
        high_reg = pair_idx * 2
        low_reg = high_reg + 1
        @registers[high_reg] = (value >> 4) & 0xF
        @registers[low_reg] = value & 0xF
      end

      # === Hardware Stack Helpers ===
      #
      # The 3-level hardware stack stores return addresses for JMS/BBL.
      # It wraps silently -- there's no stack overflow or underflow exception.
      # This was a hardware limitation of the era: 3 levels of nesting was
      # considered sufficient for calculator firmware.

      # Push a 12-bit return address onto the hardware stack.
      def stack_push(address)
        @hw_stack[@stack_pointer] = address & 0xFFF
        @stack_pointer = (@stack_pointer + 1) % 3
      end

      # Pop a 12-bit return address from the hardware stack.
      def stack_pop
        @stack_pointer = (@stack_pointer - 1) % 3
        @hw_stack[@stack_pointer]
      end

      # === RAM Access Helpers ===
      #
      # RAM addressing is set by two instructions:
      #   DCL: selects the bank (0-3)
      #   SRC: sends a register pair as address (high=register, low=character)

      def ram_read_main
        @ram[@ram_bank][@ram_register][@ram_character]
      end

      def ram_write_main(value)
        @ram[@ram_bank][@ram_register][@ram_character] = value & 0xF
      end

      def ram_read_status(index)
        @ram_status[@ram_bank][@ram_register][index]
      end

      def ram_write_status(index, value)
        @ram_status[@ram_bank][@ram_register][index] = value & 0xF
      end

      # === Instruction Dispatch ===
      #
      # The main decode logic. We first check the upper nibble to determine
      # the instruction family, then handle special cases for the 0xE_ and
      # 0xF_ ranges where the full byte is the opcode.
      def execute_instruction(raw, raw2, address)
        upper = (raw >> 4) & 0xF
        lower = raw & 0xF

        case upper
        when 0x0
          if raw == 0x00
            execute_nop
          elsif raw == 0x01
            execute_hlt
          else
            "UNKNOWN(0x#{format("%02X", raw)})"
          end
        when 0x1 then execute_jcn(lower, raw2, address)
        when 0x2
          if (lower & 0x1) == 0
            execute_fim(lower >> 1, raw2)
          else
            execute_src(lower >> 1)
          end
        when 0x3
          if (lower & 0x1) == 0
            execute_fin(lower >> 1, address)
          else
            execute_jin(lower >> 1, address)
          end
        when 0x4 then execute_jun(lower, raw2)
        when 0x5 then execute_jms(lower, raw2, address)
        when 0x6 then execute_inc(lower)
        when 0x7 then execute_isz(lower, raw2, address)
        when 0x8 then execute_add(lower)
        when 0x9 then execute_sub(lower)
        when 0xA then execute_ld(lower)
        when 0xB then execute_xch(lower)
        when 0xC then execute_bbl(lower)
        when 0xD then execute_ldm(lower)
        when 0xE then execute_io(raw)
        when 0xF then execute_accumulator_op(raw)
        else
          "UNKNOWN(0x#{format("%02X", raw)})"
        end
      end

      # -----------------------------------------------------------------
      # NOP and HLT
      # -----------------------------------------------------------------

      # NOP (0x00): No operation.
      # The simplest possible instruction -- just advance the PC (already
      # done by fetch_byte) and do nothing else.
      def execute_nop
        "NOP"
      end

      # HLT (0x01): Halt execution.
      # This is NOT a real 4004 instruction -- the real chip runs forever.
      # We add it as a simulator convenience so programs can stop cleanly.
      def execute_hlt
        @halted = true
        "HLT"
      end

      # -----------------------------------------------------------------
      # Immediate load
      # -----------------------------------------------------------------

      # LDM N (0xDN): Load immediate 4-bit value into accumulator.
      # A = N. The simplest way to get a constant into the CPU.
      def execute_ldm(n)
        @accumulator = n & 0xF
        "LDM #{n}"
      end

      # -----------------------------------------------------------------
      # Register operations
      # -----------------------------------------------------------------

      # LD Rn (0xAR): Load register into accumulator. A = Rn.
      # Unlike LDM (which loads a constant), LD copies from a register.
      def execute_ld(reg)
        @accumulator = @registers[reg] & 0xF
        "LD R#{reg}"
      end

      # XCH Rn (0xBR): Exchange accumulator with register. Swap A and Rn.
      # This is how you "store" the accumulator -- there's no dedicated
      # store instruction on the 4004.
      def execute_xch(reg)
        old_a = @accumulator
        @accumulator = @registers[reg] & 0xF
        @registers[reg] = old_a & 0xF
        "XCH R#{reg}"
      end

      # INC Rn (0x6R): Increment register. Rn = (Rn + 1) & 0xF.
      # Note: INC does NOT affect the carry flag. It's purely a register
      # increment with 4-bit wrap-around.
      def execute_inc(reg)
        @registers[reg] = (@registers[reg] + 1) & 0xF
        "INC R#{reg}"
      end

      # -----------------------------------------------------------------
      # Arithmetic (register)
      # -----------------------------------------------------------------

      # ADD Rn (0x8R): Add register to accumulator with carry.
      # A = A + Rn + carry. Carry is set if result > 15.
      #
      # The carry flag participates in the addition -- this is how
      # multi-digit BCD arithmetic works. After adding two BCD digits,
      # the carry propagates to the next digit pair.
      def execute_add(reg)
        result = @accumulator + @registers[reg] + carry_bit
        @carry = result > 0xF
        @accumulator = result & 0xF
        "ADD R#{reg}"
      end

      # SUB Rn (0x9R): Subtract register from accumulator with borrow.
      # A = A + ~Rn + borrow_in, where borrow_in = 1 if carry is clear.
      #
      # The 4004 uses complement-add for subtraction. The carry flag is
      # INVERTED from what you might expect:
      #   - carry=true  means NO borrow occurred (result >= 0)
      #   - carry=false means borrow occurred (result was negative)
      #
      # This matches the MCS-4 manual's definition. The initial carry
      # state acts as an inverse borrow-in.
      def execute_sub(reg)
        complement = (~@registers[reg]) & 0xF
        borrow_in = @carry ? 0 : 1
        result = @accumulator + complement + borrow_in
        @carry = result > 0xF
        @accumulator = result & 0xF
        "SUB R#{reg}"
      end

      # -----------------------------------------------------------------
      # Jump instructions
      # -----------------------------------------------------------------

      # JUN addr (0x4H 0xLL): Unconditional jump to 12-bit address.
      # The upper nibble of the first byte contributes the high 4 bits
      # of the address, the second byte provides the low 8 bits.
      def execute_jun(high_nibble, low_byte)
        addr = (high_nibble << 8) | low_byte
        @pc = addr & 0xFFF
        "JUN 0x#{format("%03X", addr)}"
      end

      # JCN cond,addr (0x1C 0xAA): Conditional jump.
      #
      # The condition nibble C has 4 bits:
      #   Bit 3 (0x8): INVERT -- if set, invert the final test result
      #   Bit 2 (0x4): TEST_ZERO -- test if accumulator == 0
      #   Bit 1 (0x2): TEST_CARRY -- test if carry == 1
      #   Bit 0 (0x1): TEST_PIN -- test input pin (always 0 in simulator)
      #
      # Multiple test bits can be set -- they are OR'd together. If the
      # (possibly inverted) result is true, the jump is taken.
      #
      # Examples:
      #   JCN 0x4,addr  -- jump if A == 0
      #   JCN 0xC,addr  -- jump if A != 0 (invert + test_zero)
      #   JCN 0x2,addr  -- jump if carry set
      #   JCN 0xA,addr  -- jump if carry NOT set (invert + test_carry)
      def execute_jcn(cond, addr_byte, instr_address)
        # Target is within the same 256-byte page as the instruction AFTER
        # this 2-byte JCN. The page is determined by the PC after fetching
        # both bytes (which is instr_address + 2).
        page = (instr_address + 2) & 0xF00
        target = page | addr_byte

        # Evaluate condition tests (OR'd together)
        test_result = false
        test_result = true if (cond & 0x4) != 0 && @accumulator == 0
        test_result = true if (cond & 0x2) != 0 && @carry
        # Bit 0: test pin (always 0 = not asserted, so never true)

        # Invert if bit 3 is set
        test_result = !test_result if (cond & 0x8) != 0

        @pc = target if test_result
        "JCN #{cond},#{format("%02X", addr_byte)}"
      end

      # ISZ Rn,addr (0x7R 0xAA): Increment register, skip if zero.
      #
      # Increment Rn. If Rn != 0 after increment, jump to addr.
      # If Rn == 0 (wrapped from 15), fall through to next instruction.
      #
      # This is the 4004's loop counter instruction. Load a register with
      # a negative count (in 4-bit two's complement, e.g., -4 = 12), then
      # ISZ will loop until the register wraps to 0.
      def execute_isz(reg, addr_byte, instr_address)
        page = (instr_address + 2) & 0xF00
        target = page | addr_byte

        @registers[reg] = (@registers[reg] + 1) & 0xF

        @pc = target if @registers[reg] != 0
        "ISZ R#{reg},0x#{format("%02X", addr_byte)}"
      end

      # -----------------------------------------------------------------
      # Subroutine instructions
      # -----------------------------------------------------------------

      # JMS addr (0x5H 0xLL): Jump to subroutine.
      # Push the address of the NEXT instruction onto the hardware stack,
      # then jump to the 12-bit target address.
      def execute_jms(high_nibble, low_byte, instr_address)
        addr = (high_nibble << 8) | low_byte
        # Return address is the instruction AFTER this 2-byte JMS
        return_addr = instr_address + 2
        stack_push(return_addr)
        @pc = addr & 0xFFF
        "JMS 0x#{format("%03X", addr)}"
      end

      # BBL N (0xCN): Branch back and load.
      # Pop the top of the hardware stack, load N into the accumulator,
      # and jump to the popped address.
      #
      # This is the 4004's "return from subroutine" instruction with a
      # twist -- it also loads an immediate value into A. This lets a
      # subroutine return a simple status code.
      def execute_bbl(n)
        @accumulator = n & 0xF
        return_addr = stack_pop
        @pc = return_addr & 0xFFF
        "BBL #{n}"
      end

      # -----------------------------------------------------------------
      # Register pair instructions
      # -----------------------------------------------------------------

      # FIM Pp,data (0x2P 0xDD): Fetch immediate to register pair.
      # Load the 8-bit immediate data into register pair Pp.
      # High nibble goes to R(2*p), low nibble goes to R(2*p+1).
      def execute_fim(pair, data)
        write_pair(pair, data)
        "FIM P#{pair},0x#{format("%02X", data)}"
      end

      # SRC Pp (0x2P+1): Send register control.
      # Send the 8-bit value in register pair Pp as an address for
      # subsequent RAM/ROM I/O operations. The high nibble selects the
      # RAM register (0-3), the low nibble selects the character (0-15).
      def execute_src(pair)
        pair_val = read_pair(pair)
        @ram_register = (pair_val >> 4) & 0xF
        @ram_character = pair_val & 0xF
        "SRC P#{pair}"
      end

      # FIN Pp (0x3P even): Fetch indirect from ROM.
      # Read the ROM byte at the address formed by the current page and
      # register pair P0 (R0:R1), store the result into register pair Pp.
      def execute_fin(pair, instr_address)
        # Address comes from P0 (R0:R1)
        p0_val = read_pair(0)
        # Same page as current instruction
        current_page = instr_address & 0xF00
        rom_addr = current_page | p0_val
        rom_byte = (rom_addr < @memory_size) ? @memory[rom_addr] : 0
        write_pair(pair, rom_byte)
        "FIN P#{pair}"
      end

      # JIN Pp (0x3P+1 odd): Jump indirect.
      # Jump to the address formed by the current page and register pair Pp.
      # PC[11:8] stays the same, PC[7:0] = pair value.
      def execute_jin(pair, instr_address)
        pair_val = read_pair(pair)
        current_page = instr_address & 0xF00
        @pc = current_page | pair_val
        "JIN P#{pair}"
      end

      # -----------------------------------------------------------------
      # I/O instructions (0xE0-0xEF)
      # -----------------------------------------------------------------
      #
      # These instructions move data between the accumulator and the RAM/ROM
      # I/O subsystem. The target register and character are set by a prior
      # SRC instruction; the bank is set by a prior DCL instruction.

      def execute_io(raw)
        case raw
        when 0xE0 then execute_wrm
        when 0xE1 then execute_wmp
        when 0xE2 then execute_wrr
        when 0xE3 then execute_wpm
        when 0xE4 then execute_wr(0)
        when 0xE5 then execute_wr(1)
        when 0xE6 then execute_wr(2)
        when 0xE7 then execute_wr(3)
        when 0xE8 then execute_sbm
        when 0xE9 then execute_rdm
        when 0xEA then execute_rdr
        when 0xEB then execute_adm
        when 0xEC then execute_rd(0)
        when 0xED then execute_rd(1)
        when 0xEE then execute_rd(2)
        when 0xEF then execute_rd(3)
        else
          "UNKNOWN(0x#{format("%02X", raw)})"
        end
      end

      # WRM (0xE0): Write accumulator to RAM main character.
      def execute_wrm
        ram_write_main(@accumulator)
        "WRM"
      end

      # WMP (0xE1): Write accumulator to RAM output port.
      # Each RAM bank has its own output port. WMP writes to the port
      # selected by the current bank (set by DCL).
      def execute_wmp
        @ram_output[@ram_bank] = @accumulator & 0xF
        "WMP"
      end

      # WRR (0xE2): Write accumulator to ROM I/O port.
      def execute_wrr
        @rom_port = @accumulator & 0xF
        "WRR"
      end

      # WPM (0xE3): Write program RAM (EPROM programming).
      # Not applicable in simulation -- treated as NOP.
      def execute_wpm
        "WPM"
      end

      # WR0-WR3 (0xE4-0xE7): Write accumulator to RAM status character.
      def execute_wr(index)
        ram_write_status(index, @accumulator)
        "WR#{index}"
      end

      # SBM (0xE8): Subtract RAM main character from accumulator.
      # Uses the same complement-add logic as SUB.
      def execute_sbm
        ram_val = ram_read_main
        complement = (~ram_val) & 0xF
        borrow_in = @carry ? 0 : 1
        result = @accumulator + complement + borrow_in
        @carry = result > 0xF
        @accumulator = result & 0xF
        "SBM"
      end

      # RDM (0xE9): Read RAM main character into accumulator.
      def execute_rdm
        @accumulator = ram_read_main
        "RDM"
      end

      # RDR (0xEA): Read ROM I/O port into accumulator.
      def execute_rdr
        @accumulator = @rom_port & 0xF
        "RDR"
      end

      # ADM (0xEB): Add RAM main character to accumulator with carry.
      # Same logic as ADD but reads from RAM instead of a register.
      def execute_adm
        ram_val = ram_read_main
        result = @accumulator + ram_val + carry_bit
        @carry = result > 0xF
        @accumulator = result & 0xF
        "ADM"
      end

      # RD0-RD3 (0xEC-0xEF): Read RAM status character into accumulator.
      def execute_rd(index)
        @accumulator = ram_read_status(index)
        "RD#{index}"
      end

      # -----------------------------------------------------------------
      # Accumulator operations (0xF0-0xFD)
      # -----------------------------------------------------------------
      #
      # These instructions manipulate the accumulator and carry flag
      # without involving registers or memory.

      def execute_accumulator_op(raw)
        case raw
        when 0xF0 then execute_clb
        when 0xF1 then execute_clc
        when 0xF2 then execute_iac
        when 0xF3 then execute_cmc
        when 0xF4 then execute_cma
        when 0xF5 then execute_ral
        when 0xF6 then execute_rar
        when 0xF7 then execute_tcc
        when 0xF8 then execute_dac
        when 0xF9 then execute_tcs
        when 0xFA then execute_stc
        when 0xFB then execute_daa
        when 0xFC then execute_kbp
        when 0xFD then execute_dcl
        else
          "UNKNOWN(0x#{format("%02X", raw)})"
        end
      end

      # CLB (0xF0): Clear both. A = 0, carry = false.
      def execute_clb
        @accumulator = 0
        @carry = false
        "CLB"
      end

      # CLC (0xF1): Clear carry. carry = false.
      def execute_clc
        @carry = false
        "CLC"
      end

      # IAC (0xF2): Increment accumulator. A = (A + 1) & 0xF.
      # Carry is set if A was 15 (wraps to 0).
      def execute_iac
        result = @accumulator + 1
        @carry = result > 0xF
        @accumulator = result & 0xF
        "IAC"
      end

      # CMC (0xF3): Complement carry. carry = !carry.
      def execute_cmc
        @carry = !@carry
        "CMC"
      end

      # CMA (0xF4): Complement accumulator. A = ~A & 0xF (4-bit NOT).
      # Each bit is flipped: 0b0101 becomes 0b1010.
      def execute_cma
        @accumulator = (~@accumulator) & 0xF
        "CMA"
      end

      # RAL (0xF5): Rotate accumulator left through carry.
      #
      # Before: [carry | A3 A2 A1 A0]
      # After:  [A3   | A2 A1 A0 carry_old]
      #
      # The carry shifts into the lowest bit, and the highest bit shifts
      # into carry. This is a 5-bit rotation through carry.
      def execute_ral
        old_carry = carry_bit
        @carry = (@accumulator & 0x8) != 0
        @accumulator = ((@accumulator << 1) | old_carry) & 0xF
        "RAL"
      end

      # RAR (0xF6): Rotate accumulator right through carry.
      #
      # Before: [carry | A3 A2 A1 A0]
      # After:  [A0   | carry_old A3 A2 A1]
      #
      # The carry shifts into the highest bit, and the lowest bit shifts
      # into carry. This is a 5-bit rotation through carry.
      def execute_rar
        old_carry = carry_bit
        @carry = (@accumulator & 0x1) != 0
        @accumulator = ((@accumulator >> 1) | (old_carry << 3)) & 0xF
        "RAR"
      end

      # TCC (0xF7): Transfer carry to accumulator, clear carry.
      # A = 1 if carry was set, else 0. Carry is always cleared.
      def execute_tcc
        @accumulator = @carry ? 1 : 0
        @carry = false
        "TCC"
      end

      # DAC (0xF8): Decrement accumulator. A = (A - 1) & 0xF.
      # Carry is SET if no borrow (A > 0), CLEARED if borrow (A was 0).
      def execute_dac
        result = @accumulator - 1
        @carry = result >= 0
        @accumulator = result & 0xF
        "DAC"
      end

      # TCS (0xF9): Transfer carry subtract.
      # A = 10 if carry was set, else 9. Carry is always cleared.
      #
      # Used in BCD subtraction: provides the tens-complement correction
      # factor. After complementing a BCD digit, you add TCS to get the
      # correct subtraction result.
      def execute_tcs
        @accumulator = @carry ? 10 : 9
        @carry = false
        "TCS"
      end

      # STC (0xFA): Set carry. carry = true.
      def execute_stc
        @carry = true
        "STC"
      end

      # DAA (0xFB): Decimal adjust accumulator (BCD correction).
      #
      # If A > 9 or carry is set, add 6 to A. If the addition causes
      # overflow past 15, set carry.
      #
      # This instruction exists because the 4004 was built for BCD
      # calculators. When you add two BCD digits (0-9 each), the result
      # might be > 9 (e.g., 7 + 8 = 15). DAA corrects this by adding 6,
      # wrapping to the correct BCD digit (15 + 6 = 21, keep lower nibble
      # 5, set carry for the tens digit).
      def execute_daa
        if @accumulator > 9 || @carry
          result = @accumulator + 6
          @carry = true if result > 0xF
          @accumulator = result & 0xF
        end
        "DAA"
      end

      # KBP (0xFC): Keyboard process.
      #
      # Converts a 1-hot encoded input to a binary position number:
      #   0b0000 (0)  -> 0  (no key pressed)
      #   0b0001 (1)  -> 1  (key 1)
      #   0b0010 (2)  -> 2  (key 2)
      #   0b0100 (4)  -> 3  (key 3)
      #   0b1000 (8)  -> 4  (key 4)
      #   anything else -> 15 (error: multiple keys pressed)
      #
      # This was designed for the Busicom calculator's keyboard scanning.
      KBP_TABLE = {0 => 0, 1 => 1, 2 => 2, 4 => 3, 8 => 4}.freeze

      def execute_kbp
        @accumulator = KBP_TABLE.fetch(@accumulator, 15)
        "KBP"
      end

      # DCL (0xFD): Designate command line (select RAM bank).
      # The lower 3 bits of A select the RAM bank (0-7, but only 0-3
      # are typically used since the 4004 has 4 RAM banks).
      def execute_dcl
        @ram_bank = @accumulator & 0x7
        # Clamp to valid bank range (0-3)
        @ram_bank &= 0x3 if @ram_bank > 3
        "DCL"
      end

      # Helper: convert carry flag to integer (0 or 1).
      def carry_bit
        @carry ? 1 : 0
      end
    end
  end
end
