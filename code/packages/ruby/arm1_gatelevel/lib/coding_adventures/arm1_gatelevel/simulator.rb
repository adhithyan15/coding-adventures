# frozen_string_literal: true

# ===========================================================================
# ARM1 Gate-Level Simulator
# ===========================================================================
#
# This simulator produces identical results to the behavioral ARM1 simulator,
# but routes every arithmetic and logic operation through actual gate-level
# primitives: AND, OR, XOR, NOT gates chained into adders, multiplexer
# trees for the barrel shifter, and NOR trees for flag computation.
#
# The execution flow for a single ADD instruction:
#
#   1. FETCH:   Read 32-bit instruction from memory
#   2. DECODE:  Extract bit fields (combinational logic)
#   3. CONDITION: Evaluate 4-bit condition code (gate tree)
#   4. BARREL SHIFT: Process Operand2 (5-level mux tree, ~640 gates)
#   5. ALU:     32-bit ripple-carry add (32 full adders, ~160 gates)
#   6. FLAGS:   Compute N/Z/C/V from result bits (NOR tree, XOR gates)
#   7. WRITE:   Store result in register file
#
# Total per instruction: ~1,000-1,500 gate function calls.

module CodingAdventures
  module Arm1Gatelevel
    # Shorthand aliases for frequently-used modules
    G  = LogicGates
    C  = LogicGates::Combinational
    A  = Arithmetic
    Sim = Arm1Simulator

    # =====================================================================
    # Bit Conversion Helpers
    # =====================================================================
    #
    # Converts between integer values and bit arrays (LSB-first). The ARM1
    # uses 32-bit data paths, so most conversions use width=32.
    #
    # LSB-first ordering matches how ripple-carry adders process data:
    # bit 0 feeds the first full adder, bit 1 feeds the second, etc.
    #
    #   int_to_bits(5, 32) => [1, 0, 1, 0, 0, 0, ..., 0]  (32 elements)

    # Converts a uint32 to an array of bits (LSB first).
    def self.int_to_bits(value, width = 32)
      value &= Sim::MASK32
      Array.new(width) { |i| (value >> i) & 1 }
    end

    # Converts an array of bits (LSB first) to an integer.
    def self.bits_to_int(bits)
      result = 0
      bits.each_with_index do |bit, i|
        break if i >= 32
        result |= (bit << i)
      end
      result
    end

    # =====================================================================
    # Gate-Level ALU
    # =====================================================================
    #
    # Every operation routes through actual gate function calls:
    #   - Arithmetic: ripple_carry_adder (32 full adders -> 160+ gate calls)
    #   - Logical: AND/OR/XOR/NOT applied to each of 32 bits

    # GateALUResult holds the output of a gate-level ALU operation.
    GateALUResult = Struct.new(:result, :n, :z, :c, :v, keyword_init: true)

    # Performs one of the 16 ALU operations using gate-level logic.
    #
    # All parameters are bit arrays (LSB-first) or single bits (0/1).
    def self.gate_alu_execute(opcode, a, b, carry_in, shifter_carry, old_v)
      result = nil
      carry = 0
      overflow = 0

      case opcode
      # -- Logical operations --
      when 0x0, 0x8  # AND, TST
        result = bitwise_gate(a, b) { |x, y| G.and_gate(x, y) }
        carry = shifter_carry
        overflow = old_v

      when 0x1, 0x9  # EOR, TEQ
        result = bitwise_gate(a, b) { |x, y| G.xor_gate(x, y) }
        carry = shifter_carry
        overflow = old_v

      when 0xC  # ORR
        result = bitwise_gate(a, b) { |x, y| G.or_gate(x, y) }
        carry = shifter_carry
        overflow = old_v

      when 0xD  # MOV
        result = b.dup
        carry = shifter_carry
        overflow = old_v

      when 0xE  # BIC = AND(a, NOT(b))
        not_b = bitwise_not(b)
        result = bitwise_gate(a, not_b) { |x, y| G.and_gate(x, y) }
        carry = shifter_carry
        overflow = old_v

      when 0xF  # MVN = NOT(b)
        result = bitwise_not(b)
        carry = shifter_carry
        overflow = old_v

      # -- Arithmetic operations --
      when 0x4, 0xB  # ADD, CMN
        rca = A.ripple_carry_adder(a, b, carry_in: 0)
        result = rca.bits
        carry = rca.carry
        overflow = compute_overflow(a, b, result)

      when 0x5  # ADC
        rca = A.ripple_carry_adder(a, b, carry_in: carry_in)
        result = rca.bits
        carry = rca.carry
        overflow = compute_overflow(a, b, result)

      when 0x2, 0xA  # SUB, CMP: A - B = A + NOT(B) + 1
        not_b = bitwise_not(b)
        rca = A.ripple_carry_adder(a, not_b, carry_in: 1)
        result = rca.bits
        carry = rca.carry
        overflow = compute_overflow(a, not_b, result)

      when 0x6  # SBC: A + NOT(B) + C
        not_b = bitwise_not(b)
        rca = A.ripple_carry_adder(a, not_b, carry_in: carry_in)
        result = rca.bits
        carry = rca.carry
        overflow = compute_overflow(a, not_b, result)

      when 0x3  # RSB: B - A = B + NOT(A) + 1
        not_a = bitwise_not(a)
        rca = A.ripple_carry_adder(b, not_a, carry_in: 1)
        result = rca.bits
        carry = rca.carry
        overflow = compute_overflow(b, not_a, result)

      when 0x7  # RSC: B + NOT(A) + C
        not_a = bitwise_not(a)
        rca = A.ripple_carry_adder(b, not_a, carry_in: carry_in)
        result = rca.bits
        carry = rca.carry
        overflow = compute_overflow(b, not_a, result)

      else
        result = Array.new(32, 0)
      end

      # Compute N and Z flags from result bits
      n = result[31]
      z = compute_zero(result)

      GateALUResult.new(result: result, n: n, z: z, c: carry, v: overflow)
    end

    # =====================================================================
    # Gate-Level Barrel Shifter
    # =====================================================================
    #
    # On the real ARM1, the barrel shifter was a 32x32 crossbar of pass
    # transistors. We model it with a 5-level tree of Mux2 gates.

    # Performs a shift operation on a 32-bit value using mux gates.
    # Returns [result_bits, carry_out].
    def self.gate_barrel_shift(value, shift_type, amount, carry_in, by_register)
      if by_register && amount == 0
        return [value.dup, carry_in]
      end

      case shift_type
      when 0 then gate_lsl(value, amount, carry_in, by_register)
      when 1 then gate_lsr(value, amount, carry_in, by_register)
      when 2 then gate_asr(value, amount, carry_in, by_register)
      when 3 then gate_ror(value, amount, carry_in, by_register)
      else [value.dup, carry_in]
      end
    end

    # Decodes a rotated immediate using gate-level rotation.
    def self.gate_decode_immediate(imm8, rotate)
      bits = int_to_bits(imm8, 32)
      rotate_amount = rotate * 2
      if rotate_amount == 0
        return [bits, 0]
      end
      gate_ror(bits, rotate_amount, 0, false)
    end

    # =====================================================================
    # ARM1GateLevel CPU Class
    # =====================================================================

    class ARM1GateLevel
      attr_reader :gate_ops

      def initialize(memory_size = 1024 * 1024)
        memory_size = 1024 * 1024 if memory_size <= 0
        @memory = Array.new(memory_size, 0)
        @regs = Array.new(27) { Array.new(32, 0) }
        @halted = false
        @gate_ops = 0
        reset
      end

      def reset
        @regs.each { |r| r.fill(0) }
        # Set R15: SVC mode, IRQ/FIQ disabled
        r15val = (Sim::FLAG_I | Sim::FLAG_F | Sim::MODE_SVC) & Sim::MASK32
        @regs[15] = Arm1Gatelevel.int_to_bits(r15val, 32)
        @halted = false
        @gate_ops = 0
      end

      # -------------------------------------------------------------------
      # Register access (gate-level)
      # -------------------------------------------------------------------

      def read_reg(index)
        phys = physical_reg(index)
        Arm1Gatelevel.bits_to_int(@regs[phys])
      end

      def write_reg(index, value)
        phys = physical_reg(index)
        @regs[phys] = Arm1Gatelevel.int_to_bits(value, 32)
      end

      def read_reg_bits(index)
        phys = physical_reg(index)
        @regs[phys].dup
      end

      def pc
        Arm1Gatelevel.bits_to_int(@regs[15]) & Sim::PC_MASK
      end

      def set_pc(addr)
        r15 = Arm1Gatelevel.bits_to_int(@regs[15])
        r15 = (r15 & ~Sim::PC_MASK) | (addr & Sim::PC_MASK)
        @regs[15] = Arm1Gatelevel.int_to_bits(r15 & Sim::MASK32, 32)
      end

      def flags
        r15 = @regs[15]
        Sim::Flags.new(
          n: r15[31] == 1,
          z: r15[30] == 1,
          c: r15[29] == 1,
          v: r15[28] == 1
        )
      end

      def set_flags_bits(n, z, c, v)
        @regs[15][31] = n
        @regs[15][30] = z
        @regs[15][29] = c
        @regs[15][28] = v
      end

      def mode
        Arm1Gatelevel.bits_to_int(@regs[15]) & Sim::MODE_MASK
      end

      def halted?
        @halted
      end

      # -------------------------------------------------------------------
      # Memory (same as behavioral — not gate-level)
      # -------------------------------------------------------------------

      def read_word(addr)
        addr &= Sim::PC_MASK
        a = addr & ~3
        return 0 if a + 3 >= @memory.length

        @memory[a] | (@memory[a + 1] << 8) | (@memory[a + 2] << 16) | (@memory[a + 3] << 24)
      end

      def write_word(addr, value)
        addr &= Sim::PC_MASK
        a = addr & ~3
        return if a + 3 >= @memory.length

        value &= Sim::MASK32
        @memory[a]     = value & 0xFF
        @memory[a + 1] = (value >> 8) & 0xFF
        @memory[a + 2] = (value >> 16) & 0xFF
        @memory[a + 3] = (value >> 24) & 0xFF
      end

      def read_byte(addr)
        addr &= Sim::PC_MASK
        return 0 if addr >= @memory.length

        @memory[addr]
      end

      def write_byte(addr, value)
        addr &= Sim::PC_MASK
        return if addr >= @memory.length

        @memory[addr] = value & 0xFF
      end

      def load_program(code, start_addr = 0)
        code.each_with_index do |b, i|
          addr = start_addr + i
          @memory[addr] = b & 0xFF if addr < @memory.length
        end
      end

      # -------------------------------------------------------------------
      # Condition evaluation (gate-level)
      # -------------------------------------------------------------------

      def evaluate_condition(cond, f)
        n = f.n ? 1 : 0
        z = f.z ? 1 : 0
        c = f.c ? 1 : 0
        v = f.v ? 1 : 0

        @gate_ops += 4

        case cond
        when Sim::COND_EQ then z == 1
        when Sim::COND_NE then G.not_gate(z) == 1
        when Sim::COND_CS then c == 1
        when Sim::COND_CC then G.not_gate(c) == 1
        when Sim::COND_MI then n == 1
        when Sim::COND_PL then G.not_gate(n) == 1
        when Sim::COND_VS then v == 1
        when Sim::COND_VC then G.not_gate(v) == 1
        when Sim::COND_HI then G.and_gate(c, G.not_gate(z)) == 1
        when Sim::COND_LS then G.or_gate(G.not_gate(c), z) == 1
        when Sim::COND_GE then G.xnor_gate(n, v) == 1
        when Sim::COND_LT then G.xor_gate(n, v) == 1
        when Sim::COND_GT then G.and_gate(G.not_gate(z), G.xnor_gate(n, v)) == 1
        when Sim::COND_LE then G.or_gate(z, G.xor_gate(n, v)) == 1
        when Sim::COND_AL then true
        when Sim::COND_NV then false
        else false
        end
      end

      # -------------------------------------------------------------------
      # Execution
      # -------------------------------------------------------------------

      def step
        current_pc = pc
        regs_before = Array.new(16) { |i| read_reg(i) }
        flags_before = flags

        instruction = read_word(current_pc)
        decoded = Sim.decode(instruction)
        cond_met = evaluate_condition(decoded.cond, flags_before)

        trace = Sim::Trace.new(
          address: current_pc,
          raw: instruction,
          mnemonic: Sim.disassemble(decoded),
          condition: Sim::COND_NAMES.fetch(decoded.cond, "??"),
          condition_met: cond_met,
          regs_before: regs_before,
          flags_before: flags_before,
          memory_reads: [],
          memory_writes: []
        )

        set_pc(current_pc + 4)

        if cond_met
          case decoded.type
          when Sim::INST_DATA_PROCESSING
            execute_data_processing(decoded, trace)
          when Sim::INST_LOAD_STORE
            execute_load_store(decoded, trace)
          when Sim::INST_BLOCK_TRANSFER
            execute_block_transfer(decoded, trace)
          when Sim::INST_BRANCH
            execute_branch(decoded, trace)
          when Sim::INST_SWI
            execute_swi(decoded, trace)
          when Sim::INST_COPROCESSOR, Sim::INST_UNDEFINED
            trap_undefined(current_pc)
          end
        end

        trace.regs_after = Array.new(16) { |i| read_reg(i) }
        trace.flags_after = flags
        trace
      end

      def run(max_steps)
        traces = []
        max_steps.times do
          break if @halted

          traces << step
        end
        traces
      end

      private

      def physical_reg(index)
        m = mode
        case
        when m == Sim::MODE_FIQ && index >= 8 && index <= 14
          16 + (index - 8)
        when m == Sim::MODE_IRQ && index >= 13 && index <= 14
          23 + (index - 13)
        when m == Sim::MODE_SVC && index >= 13 && index <= 14
          25 + (index - 13)
        else
          index
        end
      end

      def read_reg_for_exec(index)
        if index == 15
          (Arm1Gatelevel.bits_to_int(@regs[15]) + 4) & Sim::MASK32
        else
          read_reg(index)
        end
      end

      def read_reg_bits_for_exec(index)
        if index == 15
          val = (Arm1Gatelevel.bits_to_int(@regs[15]) + 4) & Sim::MASK32
          Arm1Gatelevel.int_to_bits(val, 32)
        else
          read_reg_bits(index)
        end
      end

      # -------------------------------------------------------------------
      # Data Processing (gate-level)
      # -------------------------------------------------------------------

      def execute_data_processing(d, trace)
        a_bits = if d.opcode != Sim::OP_MOV && d.opcode != Sim::OP_MVN
                   read_reg_bits_for_exec(d.rn)
                 else
                   Array.new(32, 0)
                 end

        f = flags
        flag_c = f.c ? 1 : 0
        flag_v = f.v ? 1 : 0

        if d.immediate
          b_bits, shifter_carry = Arm1Gatelevel.gate_decode_immediate(d.imm8, d.rotate)
          shifter_carry = flag_c if d.rotate == 0
        else
          rm_bits = read_reg_bits_for_exec(d.rm)
          shift_amount = if d.shift_by_reg
                           read_reg(d.rs) & 0xFF
                         else
                           d.shift_imm || 0
                         end
          b_bits, shifter_carry = Arm1Gatelevel.gate_barrel_shift(
            rm_bits, d.shift_type, shift_amount, flag_c, d.shift_by_reg
          )
        end

        result = Arm1Gatelevel.gate_alu_execute(d.opcode, a_bits, b_bits, flag_c, shifter_carry, flag_v)
        @gate_ops += 200

        result_val = Arm1Gatelevel.bits_to_int(result.result)

        if !Sim.test_op?(d.opcode)
          if d.rd == 15
            if d.s
              @regs[15] = Arm1Gatelevel.int_to_bits(result_val & Sim::MASK32, 32)
            else
              set_pc(result_val & Sim::PC_MASK)
            end
          else
            write_reg(d.rd, result_val)
          end
        end

        if d.s && d.rd != 15
          set_flags_bits(result.n, result.z, result.c, result.v)
        end
        if Sim.test_op?(d.opcode)
          set_flags_bits(result.n, result.z, result.c, result.v)
        end
      end

      # -------------------------------------------------------------------
      # Load/Store (gate-level register access, behavioral memory)
      # -------------------------------------------------------------------

      def execute_load_store(d, trace)
        if d.immediate
          rm_val = read_reg_for_exec(d.rm)
          if (d.shift_imm || 0) != 0
            rm_bits = Arm1Gatelevel.int_to_bits(rm_val, 32)
            flag_c = flags.c ? 1 : 0
            shifted, _ = Arm1Gatelevel.gate_barrel_shift(rm_bits, d.shift_type, d.shift_imm, flag_c, false)
            rm_val = Arm1Gatelevel.bits_to_int(shifted)
          end
          offset = rm_val
        else
          offset = d.offset12 || 0
        end

        base = read_reg_for_exec(d.rn)
        addr = d.up ? (base + offset) & Sim::MASK32 : (base - offset) & Sim::MASK32
        transfer_addr = d.pre_index ? addr : base

        if d.load
          if d.byte
            value = read_byte(transfer_addr)
          else
            value = read_word(transfer_addr)
            rotation = (transfer_addr & 3) * 8
            if rotation != 0
              value = ((value >> rotation) | (value << (32 - rotation))) & Sim::MASK32
            end
          end
          trace.memory_reads << Sim::MemoryAccess.new(address: transfer_addr, value: value)
          if d.rd == 15
            @regs[15] = Arm1Gatelevel.int_to_bits(value & Sim::MASK32, 32)
          else
            write_reg(d.rd, value)
          end
        else
          value = read_reg_for_exec(d.rd)
          if d.byte
            write_byte(transfer_addr, value & 0xFF)
          else
            write_word(transfer_addr, value)
          end
          trace.memory_writes << Sim::MemoryAccess.new(address: transfer_addr, value: value)
        end

        if d.write_back || !d.pre_index
          write_reg(d.rn, addr) if d.rn != 15
        end
      end

      # -------------------------------------------------------------------
      # Block Transfer (LDM/STM)
      # -------------------------------------------------------------------

      def execute_block_transfer(d, trace)
        base = read_reg(d.rn)
        reg_list = d.register_list || 0
        count = 0
        16.times { |i| count += 1 if ((reg_list >> i) & 1) == 1 }
        return if count == 0

        start_addr = case [d.pre_index, d.up]
                     when [false, true]  then base
                     when [true, true]   then base + 4
                     when [false, false] then base - (count * 4) + 4
                     when [true, false]  then base - (count * 4)
                     end
        start_addr &= Sim::MASK32

        addr = start_addr
        16.times do |i|
          next if ((reg_list >> i) & 1) == 0

          if d.load
            value = read_word(addr)
            trace.memory_reads << Sim::MemoryAccess.new(address: addr, value: value)
            if i == 15
              @regs[15] = Arm1Gatelevel.int_to_bits(value & Sim::MASK32, 32)
            else
              write_reg(i, value)
            end
          else
            value = if i == 15
                      (Arm1Gatelevel.bits_to_int(@regs[15]) + 4) & Sim::MASK32
                    else
                      read_reg(i)
                    end
            write_word(addr, value)
            trace.memory_writes << Sim::MemoryAccess.new(address: addr, value: value)
          end
          addr = (addr + 4) & Sim::MASK32
        end

        if d.write_back
          new_base = d.up ? (base + count * 4) & Sim::MASK32 : (base - count * 4) & Sim::MASK32
          write_reg(d.rn, new_base)
        end
      end

      # -------------------------------------------------------------------
      # Branch
      # -------------------------------------------------------------------

      def execute_branch(d, trace)
        branch_base = (pc + 4) & Sim::MASK32
        if d.link
          return_addr = Arm1Gatelevel.bits_to_int(@regs[15]) & Sim::MASK32
          write_reg(14, return_addr)
        end
        target = (branch_base + d.branch_offset) & Sim::MASK32
        set_pc(target & Sim::PC_MASK)
      end

      # -------------------------------------------------------------------
      # SWI
      # -------------------------------------------------------------------

      def execute_swi(d, trace)
        if d.swi_comment == Sim::HALT_SWI
          @halted = true
          return
        end

        r15val = Arm1Gatelevel.bits_to_int(@regs[15])
        @regs[25] = @regs[15].dup
        @regs[26] = @regs[15].dup

        r15val = (r15val & ~Sim::MODE_MASK) | Sim::MODE_SVC
        r15val |= Sim::FLAG_I
        @regs[15] = Arm1Gatelevel.int_to_bits(r15val & Sim::MASK32, 32)
        set_pc(0x08)
      end

      # -------------------------------------------------------------------
      # Exception
      # -------------------------------------------------------------------

      def trap_undefined(_instr_addr)
        @regs[26] = @regs[15].dup
        r15val = Arm1Gatelevel.bits_to_int(@regs[15])
        r15val = (r15val & ~Sim::MODE_MASK) | Sim::MODE_SVC
        r15val |= Sim::FLAG_I
        @regs[15] = Arm1Gatelevel.int_to_bits(r15val & Sim::MASK32, 32)
        set_pc(0x04)
      end
    end

    # =====================================================================
    # Private helper methods for gate-level operations
    # =====================================================================

    # Applies a 2-input gate function to each bit pair.
    def self.bitwise_gate(a, b, &block)
      Array.new(a.length) { |i| block.call(a[i], b[i]) }
    end

    # Applies NOT to each bit.
    def self.bitwise_not(bits)
      bits.map { |b| G.not_gate(b) }
    end

    # Checks if all bits are zero using NOR gates (OR tree + NOT).
    def self.compute_zero(bits)
      combined = bits[0]
      (1...bits.length).each do |i|
        combined = G.or_gate(combined, bits[i])
      end
      G.not_gate(combined)
    end

    # Detects signed overflow: both inputs same sign, result differs.
    def self.compute_overflow(a, b, result)
      xor1 = G.xor_gate(a[31], result[31])
      xor2 = G.xor_gate(b[31], result[31])
      G.and_gate(xor1, xor2)
    end

    # --- LSL using mux tree ---
    def self.gate_lsl(value, amount, carry_in, _by_register)
      if amount == 0
        return [value.dup, carry_in]
      end
      if amount >= 32
        result = Array.new(32, 0)
        return amount == 32 ? [result, value[0]] : [result, 0]
      end

      current = value.dup
      5.times do |level|
        shift = 1 << level
        sel = (amount >> level) & 1
        next_val = Array.new(32) do |i|
          shifted = i >= shift ? current[i - shift] : 0
          C.mux2(current[i], shifted, sel)
        end
        current = next_val
      end

      carry = (amount > 0 && amount <= 32) ? value[32 - amount] : carry_in
      [current, carry]
    end

    # --- LSR using mux tree ---
    def self.gate_lsr(value, amount, carry_in, by_register)
      if amount == 0 && !by_register
        return [Array.new(32, 0), value[31]]
      end
      if amount == 0
        return [value.dup, carry_in]
      end
      if amount >= 32
        result = Array.new(32, 0)
        return amount == 32 ? [result, value[31]] : [result, 0]
      end

      current = value.dup
      5.times do |level|
        shift = 1 << level
        sel = (amount >> level) & 1
        next_val = Array.new(32) do |i|
          shifted = (i + shift < 32) ? current[i + shift] : 0
          C.mux2(current[i], shifted, sel)
        end
        current = next_val
      end

      [current, value[amount - 1]]
    end

    # --- ASR using mux tree (sign-extending) ---
    def self.gate_asr(value, amount, carry_in, by_register)
      sign_bit = value[31]

      if amount == 0 && !by_register
        result = Array.new(32, sign_bit)
        return [result, sign_bit]
      end
      if amount == 0
        return [value.dup, carry_in]
      end
      if amount >= 32
        result = Array.new(32, sign_bit)
        return [result, sign_bit]
      end

      current = value.dup
      5.times do |level|
        shift = 1 << level
        sel = (amount >> level) & 1
        next_val = Array.new(32) do |i|
          shifted = (i + shift < 32) ? current[i + shift] : sign_bit
          C.mux2(current[i], shifted, sel)
        end
        current = next_val
      end

      [current, value[amount - 1]]
    end

    # --- ROR using mux tree ---
    def self.gate_ror(value, amount, carry_in, by_register)
      if amount == 0 && !by_register
        # RRX: 33-bit rotate through carry
        result = Array.new(32) do |i|
          i < 31 ? value[i + 1] : carry_in
        end
        return [result, value[0]]
      end
      if amount == 0
        return [value.dup, carry_in]
      end

      amount &= 31
      if amount == 0
        return [value.dup, value[31]]
      end

      current = value.dup
      5.times do |level|
        shift = 1 << level
        sel = (amount >> level) & 1
        next_val = Array.new(32) do |i|
          shifted = current[(i + shift) % 32]
          C.mux2(current[i], shifted, sel)
        end
        current = next_val
      end

      [current, current[31]]
    end

    private_class_method :bitwise_gate, :bitwise_not, :compute_zero, :compute_overflow,
                         :gate_lsl, :gate_lsr, :gate_asr, :gate_ror
  end
end
