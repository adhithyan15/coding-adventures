# frozen_string_literal: true

# ===========================================================================
# ARM1 Behavioral Simulator — Complete ARMv1 Instruction Set
# ===========================================================================
#
# The ARM1 was designed by Sophie Wilson and Steve Furber at Acorn Computers
# in Cambridge, UK. First silicon powered on April 26, 1985 — and worked
# correctly on the very first attempt. This file implements a behavioral
# simulator for the complete ARMv1 instruction set.
#
# Architecture Summary:
#   - 32-bit RISC processor, 25,000 transistors
#   - 16 visible registers (R0-R15), 25 physical (banked for FIQ/IRQ/SVC)
#   - R15 = combined Program Counter + Status Register
#   - 3-stage pipeline: Fetch -> Decode -> Execute
#   - Every instruction is conditional (4-bit condition code)
#   - Inline barrel shifter on Operand2 (shift for free)
#   - 26-bit address space (64 MiB)
#
# R15: The Combined PC + Status Register
#
# ARMv1's most distinctive architectural feature is that the program counter
# and processor status flags share a single 32-bit register (R15):
#
#   Bit 31: N (Negative)     Bit 27: I (IRQ disable)
#   Bit 30: Z (Zero)         Bit 26: F (FIQ disable)
#   Bit 29: C (Carry)        Bits 25:2: Program Counter (24 bits)
#   Bit 28: V (Overflow)     Bits 1:0: Processor Mode

module CodingAdventures
  module Arm1Simulator
    # =====================================================================
    # Processor Modes
    # =====================================================================
    #
    # The ARM1 supports 4 processor modes. Each mode has its own banked
    # copies of certain registers, allowing fast context switching.
    #
    #   Mode  M1:M0  Banked Registers
    #   ----  -----  ----------------
    #   USR   0b00   (none — base set)
    #   FIQ   0b01   R8_fiq..R12_fiq, R13_fiq, R14_fiq
    #   IRQ   0b10   R13_irq, R14_irq
    #   SVC   0b11   R13_svc, R14_svc

    MODE_USR = 0  # User mode — normal program execution
    MODE_FIQ = 1  # Fast Interrupt — banks R8-R14 for zero-overhead handlers
    MODE_IRQ = 2  # Normal Interrupt — banks R13-R14
    MODE_SVC = 3  # Supervisor — entered via SWI or Reset

    # Human-readable name for a processor mode.
    MODE_NAMES = {
      MODE_USR => "USR",
      MODE_FIQ => "FIQ",
      MODE_IRQ => "IRQ",
      MODE_SVC => "SVC"
    }.freeze

    # =====================================================================
    # Condition Codes
    # =====================================================================
    #
    # Every ARM instruction has a 4-bit condition code in bits 31:28.
    # The instruction only executes if the condition is met.

    COND_EQ = 0x0  # Equal — Z set
    COND_NE = 0x1  # Not equal — Z clear
    COND_CS = 0x2  # Carry set / unsigned higher or same
    COND_CC = 0x3  # Carry clear / unsigned lower
    COND_MI = 0x4  # Minus / negative — N set
    COND_PL = 0x5  # Plus / positive or zero — N clear
    COND_VS = 0x6  # Overflow set
    COND_VC = 0x7  # Overflow clear
    COND_HI = 0x8  # Unsigned higher — C set AND Z clear
    COND_LS = 0x9  # Unsigned lower or same — C clear OR Z set
    COND_GE = 0xA  # Signed greater or equal — N == V
    COND_LT = 0xB  # Signed less than — N != V
    COND_GT = 0xC  # Signed greater than — Z clear AND N == V
    COND_LE = 0xD  # Signed less or equal — Z set OR N != V
    COND_AL = 0xE  # Always (unconditional)
    COND_NV = 0xF  # Never (reserved — do not use)

    COND_NAMES = {
      COND_EQ => "EQ", COND_NE => "NE", COND_CS => "CS", COND_CC => "CC",
      COND_MI => "MI", COND_PL => "PL", COND_VS => "VS", COND_VC => "VC",
      COND_HI => "HI", COND_LS => "LS", COND_GE => "GE", COND_LT => "LT",
      COND_GT => "GT", COND_LE => "LE", COND_AL => "",   COND_NV => "NV"
    }.freeze

    # =====================================================================
    # ALU Opcodes
    # =====================================================================
    #
    # The ARM1's ALU supports 16 operations, selected by bits 24:21.

    OP_AND = 0x0  # Rd = Rn AND Op2
    OP_EOR = 0x1  # Rd = Rn XOR Op2
    OP_SUB = 0x2  # Rd = Rn - Op2
    OP_RSB = 0x3  # Rd = Op2 - Rn
    OP_ADD = 0x4  # Rd = Rn + Op2
    OP_ADC = 0x5  # Rd = Rn + Op2 + Carry
    OP_SBC = 0x6  # Rd = Rn - Op2 - NOT(Carry)
    OP_RSC = 0x7  # Rd = Op2 - Rn - NOT(Carry)
    OP_TST = 0x8  # Rn AND Op2, flags only
    OP_TEQ = 0x9  # Rn XOR Op2, flags only
    OP_CMP = 0xA  # Rn - Op2, flags only
    OP_CMN = 0xB  # Rn + Op2, flags only
    OP_ORR = 0xC  # Rd = Rn OR Op2
    OP_MOV = 0xD  # Rd = Op2
    OP_BIC = 0xE  # Rd = Rn AND NOT(Op2)
    OP_MVN = 0xF  # Rd = NOT(Op2)

    OP_NAMES = %w[AND EOR SUB RSB ADD ADC SBC RSC TST TEQ CMP CMN ORR MOV BIC MVN].freeze

    # =====================================================================
    # Shift Types
    # =====================================================================
    #
    # The barrel shifter supports 4 shift types, encoded in bits 6:5.

    SHIFT_LSL = 0  # Logical Shift Left
    SHIFT_LSR = 1  # Logical Shift Right
    SHIFT_ASR = 2  # Arithmetic Shift Right (sign-extending)
    SHIFT_ROR = 3  # Rotate Right (ROR #0 encodes RRX)

    SHIFT_NAMES = %w[LSL LSR ASR ROR].freeze

    # =====================================================================
    # R15 bit positions
    # =====================================================================

    FLAG_N    = 1 << 31  # Negative flag
    FLAG_Z    = 1 << 30  # Zero flag
    FLAG_C    = 1 << 29  # Carry flag
    FLAG_V    = 1 << 28  # Overflow flag
    FLAG_I    = 1 << 27  # IRQ disable
    FLAG_F    = 1 << 26  # FIQ disable
    PC_MASK   = 0x03FFFFFC  # Bits 25:2 — 24-bit PC field
    MODE_MASK = 0x3         # Bits 1:0 — processor mode
    MASK32    = 0xFFFFFFFF  # 32-bit mask (Ruby integers auto-promote to Bignum)

    # HaltSWI is the SWI comment field we use as a halt instruction.
    HALT_SWI = 0x123456

    # =====================================================================
    # Instruction Types
    # =====================================================================

    INST_DATA_PROCESSING = 0
    INST_LOAD_STORE      = 1
    INST_BLOCK_TRANSFER  = 2
    INST_BRANCH          = 3
    INST_SWI             = 4
    INST_COPROCESSOR     = 5
    INST_UNDEFINED       = 6

    # =====================================================================
    # Data Structures
    # =====================================================================

    # Flags represents the ARM1's four condition flags.
    Flags = Struct.new(:n, :z, :c, :v, keyword_init: true) do
      def initialize(n: false, z: false, c: false, v: false)
        super(n: n, z: z, c: c, v: v)
      end
    end

    # MemoryAccess records a single memory read or write.
    MemoryAccess = Struct.new(:address, :value, keyword_init: true)

    # ALUResult holds the output of an ALU operation.
    ALUResult = Struct.new(:result, :n, :z, :c, :v, :write_result, keyword_init: true)

    # Trace records the state change caused by executing one instruction.
    Trace = Struct.new(
      :address, :raw, :mnemonic, :condition, :condition_met,
      :regs_before, :regs_after, :flags_before, :flags_after,
      :memory_reads, :memory_writes,
      keyword_init: true
    )

    # DecodedInstruction holds all fields extracted from a 32-bit instruction.
    DecodedInstruction = Struct.new(
      :raw, :type, :cond,
      # Data Processing fields
      :opcode, :s, :rn, :rd, :immediate,
      :imm8, :rotate,
      :rm, :shift_type, :shift_by_reg, :shift_imm, :rs,
      # Load/Store fields
      :load, :byte, :pre_index, :up, :write_back, :offset12,
      # Block Transfer fields
      :register_list, :force_user,
      # Branch fields
      :link, :branch_offset,
      # SWI fields
      :swi_comment,
      keyword_init: true
    )

    # =====================================================================
    # Module Functions — Predicate helpers
    # =====================================================================

    # Returns true if the ALU opcode is a test-only operation
    # (TST, TEQ, CMP, CMN) that does not write to the destination register.
    def self.test_op?(opcode)
      opcode >= OP_TST && opcode <= OP_CMN
    end

    # Returns true if the ALU opcode is a logical operation.
    # For logical ops, the C flag comes from the barrel shifter carry-out
    # rather than the ALU's adder carry.
    def self.logical_op?(opcode)
      [OP_AND, OP_EOR, OP_TST, OP_TEQ, OP_ORR, OP_MOV, OP_BIC, OP_MVN].include?(opcode)
    end

    # =====================================================================
    # Condition Evaluator
    # =====================================================================
    #
    # Every ARM instruction has a 4-bit condition code in bits 31:28. The
    # instruction only executes if the condition is satisfied by the current
    # flags (N, Z, C, V).

    def self.evaluate_condition(cond, flags)
      case cond
      when COND_EQ then flags.z
      when COND_NE then !flags.z
      when COND_CS then flags.c
      when COND_CC then !flags.c
      when COND_MI then flags.n
      when COND_PL then !flags.n
      when COND_VS then flags.v
      when COND_VC then !flags.v
      when COND_HI then flags.c && !flags.z
      when COND_LS then !flags.c || flags.z
      when COND_GE then flags.n == flags.v
      when COND_LT then flags.n != flags.v
      when COND_GT then !flags.z && (flags.n == flags.v)
      when COND_LE then flags.z || (flags.n != flags.v)
      when COND_AL then true
      when COND_NV then false
      else false
      end
    end

    # =====================================================================
    # Barrel Shifter
    # =====================================================================
    #
    # The barrel shifter is the ARM1's most distinctive hardware feature.
    # It allows one operand to be shifted or rotated FOR FREE as part of
    # any data processing instruction.
    #
    # Returns [result, carry_out] as a two-element array.

    def self.barrel_shift(value, shift_type, amount, carry_in, by_register)
      # When shifting by a register value, if the amount is 0 the value
      # passes through unchanged and the carry is unaffected.
      if by_register && amount == 0
        return [value & MASK32, carry_in]
      end

      case shift_type
      when SHIFT_LSL then shift_lsl(value, amount, carry_in, by_register)
      when SHIFT_LSR then shift_lsr(value, amount, carry_in, by_register)
      when SHIFT_ASR then shift_asr(value, amount, carry_in, by_register)
      when SHIFT_ROR then shift_ror(value, amount, carry_in, by_register)
      else [value & MASK32, carry_in]
      end
    end

    # Logical Shift Left
    def self.shift_lsl(value, amount, carry_in, _by_register)
      value &= MASK32
      if amount == 0
        return [value, carry_in]
      end
      if amount >= 32
        if amount == 32
          return [0, (value & 1) != 0]
        end
        return [0, false]
      end
      carry = ((value >> (32 - amount)) & 1) != 0
      [(value << amount) & MASK32, carry]
    end

    # Logical Shift Right
    def self.shift_lsr(value, amount, carry_in, by_register)
      value &= MASK32
      if amount == 0 && !by_register
        # Immediate LSR #0 encodes LSR #32
        return [0, (value >> 31) != 0]
      end
      if amount == 0
        return [value, carry_in]
      end
      if amount >= 32
        if amount == 32
          return [0, (value >> 31) != 0]
        end
        return [0, false]
      end
      carry = ((value >> (amount - 1)) & 1) != 0
      [value >> amount, carry]
    end

    # Arithmetic Shift Right — sign-extending
    #
    # The sign bit (bit 31) is replicated into vacated positions.
    def self.shift_asr(value, amount, carry_in, by_register)
      value &= MASK32
      sign_bit = (value >> 31) != 0

      if amount == 0 && !by_register
        # Immediate ASR #0 encodes ASR #32
        return sign_bit ? [MASK32, true] : [0, false]
      end
      if amount == 0
        return [value, carry_in]
      end
      if amount >= 32
        return sign_bit ? [MASK32, true] : [0, false]
      end

      # Ruby's right shift on positive integers is logical. For arithmetic
      # shift, we need to convert to a signed interpretation.
      signed = value >= 0x80000000 ? (value - 0x100000000) : value
      result = (signed >> amount) & MASK32
      carry = ((value >> (amount - 1)) & 1) != 0
      [result, carry]
    end

    # Rotate Right
    #
    # Special case: immediate ROR #0 encodes RRX (Rotate Right Extended):
    # 33-bit rotation through carry flag.
    def self.shift_ror(value, amount, carry_in, by_register)
      value &= MASK32
      if amount == 0 && !by_register
        # RRX — Rotate Right Extended (33-bit rotation through carry)
        carry = (value & 1) != 0
        result = value >> 1
        result |= 0x80000000 if carry_in
        return [result, carry]
      end
      if amount == 0
        return [value, carry_in]
      end

      # Normalize rotation amount to 0-31
      amount &= 31
      if amount == 0
        # ROR by 32 (or multiple of 32): value unchanged, carry = bit 31
        return [value, (value >> 31) != 0]
      end

      result = ((value >> amount) | (value << (32 - amount))) & MASK32
      carry = ((result >> 31) & 1) != 0
      [result, carry]
    end

    private_class_method :shift_lsl, :shift_lsr, :shift_asr, :shift_ror

    # Decodes a rotated immediate value from the Operand2 field.
    #
    # The 8-bit value is rotated right by an even number of positions.
    # Returns [value, carry_out].
    def self.decode_immediate(imm8, rotate)
      rotate_amount = rotate * 2
      if rotate_amount == 0
        return [imm8 & MASK32, false]
      end
      value = ((imm8 >> rotate_amount) | (imm8 << (32 - rotate_amount))) & MASK32
      carry_out = (value >> 31) != 0
      [value, carry_out]
    end

    # =====================================================================
    # ALU — 32-bit Arithmetic Logic Unit
    # =====================================================================
    #
    # Flag computation:
    #   Arithmetic ops: C = carry out from 32-bit adder, V = signed overflow
    #   Logical ops: C = carry from barrel shifter, V = unchanged

    def self.alu_execute(opcode, a, b, carry_in, shifter_carry, old_v)
      a &= MASK32
      b &= MASK32
      write_result = !test_op?(opcode)

      case opcode
      when OP_AND, OP_TST
        result = a & b
        carry = shifter_carry
        overflow = old_v
      when OP_EOR, OP_TEQ
        result = a ^ b
        carry = shifter_carry
        overflow = old_v
      when OP_ORR
        result = a | b
        carry = shifter_carry
        overflow = old_v
      when OP_MOV
        result = b
        carry = shifter_carry
        overflow = old_v
      when OP_BIC
        result = a & (~b & MASK32)
        carry = shifter_carry
        overflow = old_v
      when OP_MVN
        result = ~b & MASK32
        carry = shifter_carry
        overflow = old_v
      when OP_ADD, OP_CMN
        result, carry, overflow = add32(a, b, false)
      when OP_ADC
        result, carry, overflow = add32(a, b, carry_in)
      when OP_SUB, OP_CMP
        result, carry, overflow = add32(a, ~b & MASK32, true)
      when OP_SBC
        result, carry, overflow = add32(a, ~b & MASK32, carry_in)
      when OP_RSB
        result, carry, overflow = add32(b, ~a & MASK32, true)
      when OP_RSC
        result, carry, overflow = add32(b, ~a & MASK32, carry_in)
      else
        result = 0
        carry = false
        overflow = false
      end

      result &= MASK32

      ALUResult.new(
        result: result,
        n: (result >> 31) != 0,
        z: result == 0,
        c: carry,
        v: overflow,
        write_result: write_result
      )
    end

    # 32-bit addition with carry-in. Returns [result, carry, overflow].
    #
    # We compute using Ruby's arbitrary-precision integers, then truncate.
    # Overflow detection: both operands have the same sign, but the result
    # has a different sign.
    def self.add32(a, b, carry_in)
      cin = carry_in ? 1 : 0
      sum = a + b + cin
      result = sum & MASK32
      carry = (sum >> 32) != 0

      # Overflow: ((a ^ result) & (b ^ result)) >> 31
      overflow = (((a ^ result) & (b ^ result)) >> 31) != 0
      [result, carry, overflow]
    end

    private_class_method :add32

    # =====================================================================
    # Instruction Decoder
    # =====================================================================
    #
    # Extracts all fields from a 32-bit ARM instruction word.

    def self.decode(instruction)
      instruction &= MASK32

      d = DecodedInstruction.new(
        raw: instruction,
        cond: (instruction >> 28) & 0xF,
        s: false,
        immediate: false,
        shift_by_reg: false,
        load: false,
        byte: false,
        pre_index: false,
        up: false,
        write_back: false,
        force_user: false,
        link: false
      )

      bits2726 = (instruction >> 26) & 0x3
      bit25 = (instruction >> 25) & 0x1

      case
      when bits2726 == 0
        d.type = INST_DATA_PROCESSING
        decode_data_processing(d, instruction)
      when bits2726 == 1
        d.type = INST_LOAD_STORE
        decode_load_store(d, instruction)
      when bits2726 == 2 && bit25 == 0
        d.type = INST_BLOCK_TRANSFER
        decode_block_transfer(d, instruction)
      when bits2726 == 2 && bit25 == 1
        d.type = INST_BRANCH
        decode_branch(d, instruction)
      when bits2726 == 3
        if ((instruction >> 24) & 0xF) == 0xF
          d.type = INST_SWI
          d.swi_comment = instruction & 0x00FFFFFF
        else
          d.type = INST_COPROCESSOR
        end
      else
        d.type = INST_UNDEFINED
      end

      d
    end

    def self.decode_data_processing(d, inst)
      d.immediate = ((inst >> 25) & 1) == 1
      d.opcode = (inst >> 21) & 0xF
      d.s = ((inst >> 20) & 1) == 1
      d.rn = (inst >> 16) & 0xF
      d.rd = (inst >> 12) & 0xF

      if d.immediate
        d.imm8 = inst & 0xFF
        d.rotate = (inst >> 8) & 0xF
      else
        d.rm = inst & 0xF
        d.shift_type = (inst >> 5) & 0x3
        d.shift_by_reg = ((inst >> 4) & 1) == 1
        if d.shift_by_reg
          d.rs = (inst >> 8) & 0xF
        else
          d.shift_imm = (inst >> 7) & 0x1F
        end
      end
    end

    def self.decode_load_store(d, inst)
      # Note: for LDR/STR, I=1 means REGISTER offset (opposite of data processing!)
      d.immediate = ((inst >> 25) & 1) == 1
      d.pre_index = ((inst >> 24) & 1) == 1
      d.up = ((inst >> 23) & 1) == 1
      d.byte = ((inst >> 22) & 1) == 1
      d.write_back = ((inst >> 21) & 1) == 1
      d.load = ((inst >> 20) & 1) == 1
      d.rn = (inst >> 16) & 0xF
      d.rd = (inst >> 12) & 0xF

      if d.immediate
        # Register offset
        d.rm = inst & 0xF
        d.shift_type = (inst >> 5) & 0x3
        d.shift_imm = (inst >> 7) & 0x1F
      else
        d.offset12 = inst & 0xFFF
      end
    end

    def self.decode_block_transfer(d, inst)
      d.pre_index = ((inst >> 24) & 1) == 1
      d.up = ((inst >> 23) & 1) == 1
      d.force_user = ((inst >> 22) & 1) == 1
      d.write_back = ((inst >> 21) & 1) == 1
      d.load = ((inst >> 20) & 1) == 1
      d.rn = (inst >> 16) & 0xF
      d.register_list = inst & 0xFFFF
    end

    def self.decode_branch(d, inst)
      d.link = ((inst >> 24) & 1) == 1

      # The 24-bit offset is sign-extended to 32 bits, then shifted left 2
      offset = inst & 0x00FFFFFF
      # Sign-extend from 24 bits
      if (offset >> 23) != 0
        offset |= 0xFF000000
      end
      # Convert to signed and shift left 2
      signed_offset = offset >= 0x80000000 ? (offset - 0x100000000) : offset
      d.branch_offset = signed_offset << 2
    end

    private_class_method :decode_data_processing, :decode_load_store,
                         :decode_block_transfer, :decode_branch

    # =====================================================================
    # Disassembly
    # =====================================================================

    def self.disassemble(d)
      cond = COND_NAMES.fetch(d.cond, "??")

      case d.type
      when INST_DATA_PROCESSING
        disasm_data_processing(d, cond)
      when INST_LOAD_STORE
        disasm_load_store(d, cond)
      when INST_BLOCK_TRANSFER
        disasm_block_transfer(d, cond)
      when INST_BRANCH
        disasm_branch(d, cond)
      when INST_SWI
        if d.swi_comment == HALT_SWI
          "HLT#{cond}"
        else
          format("SWI%s #0x%X", cond, d.swi_comment)
        end
      when INST_COPROCESSOR
        format("CDP%s (undefined)", cond)
      else
        format("UND%s #0x%08X", cond, d.raw)
      end
    end

    def self.disasm_data_processing(d, cond)
      op = OP_NAMES[d.opcode] || "???"
      suf = (d.s && !test_op?(d.opcode)) ? "S" : ""
      op2 = disasm_operand2(d)

      case
      when d.opcode == OP_MOV || d.opcode == OP_MVN
        format("%s%s%s R%d, %s", op, cond, suf, d.rd, op2)
      when test_op?(d.opcode)
        format("%s%s R%d, %s", op, cond, d.rn, op2)
      else
        format("%s%s%s R%d, R%d, %s", op, cond, suf, d.rd, d.rn, op2)
      end
    end

    def self.disasm_operand2(d)
      if d.immediate
        val, _ = decode_immediate(d.imm8, d.rotate)
        return format("#%d", val)
      end
      if !d.shift_by_reg && (d.shift_imm || 0) == 0 && d.shift_type == SHIFT_LSL
        return format("R%d", d.rm)
      end
      if d.shift_by_reg
        return format("R%d, %s R%d", d.rm, SHIFT_NAMES[d.shift_type], d.rs)
      end
      amount = d.shift_imm || 0
      if amount == 0
        case d.shift_type
        when SHIFT_LSR, SHIFT_ASR
          amount = 32
        when SHIFT_ROR
          return format("R%d, RRX", d.rm)
        end
      end
      format("R%d, %s #%d", d.rm, SHIFT_NAMES[d.shift_type], amount)
    end

    def self.disasm_load_store(d, cond)
      op = d.load ? "LDR" : "STR"
      b_suf = d.byte ? "B" : ""

      if d.immediate
        offset = format("R%d", d.rm)
        if (d.shift_imm || 0) != 0
          offset += format(", %s #%d", SHIFT_NAMES[d.shift_type], d.shift_imm)
        end
      else
        offset = format("#%d", d.offset12)
      end

      sign = d.up ? "" : "-"

      if d.pre_index
        wb = d.write_back ? "!" : ""
        format("%s%s%s R%d, [R%d, %s%s]%s", op, cond, b_suf, d.rd, d.rn, sign, offset, wb)
      else
        format("%s%s%s R%d, [R%d], %s%s", op, cond, b_suf, d.rd, d.rn, sign, offset)
      end
    end

    def self.disasm_block_transfer(d, cond)
      op = d.load ? "LDM" : "STM"
      mode = case [d.pre_index, d.up]
             when [false, true]  then "IA"
             when [true, true]   then "IB"
             when [false, false] then "DA"
             when [true, false]  then "DB"
             end
      wb = d.write_back ? "!" : ""
      regs = disasm_reg_list(d.register_list)
      format("%s%s%s R%d%s, {%s}", op, cond, mode, d.rn, wb, regs)
    end

    def self.disasm_branch(d, cond)
      op = d.link ? "BL" : "B"
      format("%s%s #%d", op, cond, d.branch_offset)
    end

    def self.disasm_reg_list(list)
      parts = []
      16.times do |i|
        if ((list >> i) & 1) == 1
          name = case i
                 when 15 then "PC"
                 when 14 then "LR"
                 when 13 then "SP"
                 else "R#{i}"
                 end
          parts << name
        end
      end
      parts.join(", ")
    end

    private_class_method :disasm_data_processing, :disasm_operand2,
                         :disasm_load_store, :disasm_block_transfer,
                         :disasm_branch, :disasm_reg_list

    # =====================================================================
    # Encoding helpers — Convenience methods for building programs
    # =====================================================================

    def self.encode_data_processing(cond, opcode, s, rn, rd, operand2)
      ((cond << 28) | operand2 | (opcode << 21) | (s << 20) | (rn << 16) | (rd << 12)) & MASK32
    end

    def self.encode_mov_imm(cond, rd, imm8)
      encode_data_processing(cond, OP_MOV, 0, 0, rd, (1 << 25) | imm8)
    end

    def self.encode_alu_reg(cond, opcode, s, rd, rn, rm)
      encode_data_processing(cond, opcode, s, rn, rd, rm)
    end

    def self.encode_branch(cond, link, offset)
      inst = (cond << 28) | 0x0A000000
      inst |= 0x01000000 if link
      encoded = (offset >> 2) & 0x00FFFFFF
      (inst | encoded) & MASK32
    end

    def self.encode_halt
      ((COND_AL << 28) | 0x0F000000 | HALT_SWI) & MASK32
    end

    def self.encode_ldr(cond, rd, rn, offset, pre_index)
      inst = (cond << 28) | 0x04100000  # bits 27:26=01, L=1, I=0
      inst |= rd << 12
      inst |= rn << 16
      inst |= (1 << 24) if pre_index
      if offset >= 0
        inst |= (1 << 23)
        inst |= offset & 0xFFF
      else
        inst |= (-offset) & 0xFFF
      end
      inst & MASK32
    end

    def self.encode_str(cond, rd, rn, offset, pre_index)
      inst = (cond << 28) | 0x04000000  # bits 27:26=01, L=0, I=0
      inst |= rd << 12
      inst |= rn << 16
      inst |= (1 << 24) if pre_index
      if offset >= 0
        inst |= (1 << 23)
        inst |= offset & 0xFFF
      else
        inst |= (-offset) & 0xFFF
      end
      inst & MASK32
    end

    def self.encode_ldm(cond, rn, reg_list, write_back, mode)
      inst = (cond << 28) | 0x08100000  # bits 27:25=100, L=1
      inst |= rn << 16
      inst |= reg_list & 0xFFFF
      inst |= (1 << 21) if write_back
      case mode
      when "IA"
        inst |= (1 << 23)  # P=0, U=1
      when "IB"
        inst |= (1 << 24) | (1 << 23)  # P=1, U=1
      when "DA"
        # P=0, U=0 (both already 0)
      when "DB"
        inst |= (1 << 24)  # P=1, U=0
      end
      inst & MASK32
    end

    def self.encode_stm(cond, rn, reg_list, write_back, mode)
      inst = encode_ldm(cond, rn, reg_list, write_back, mode)
      inst &= ~(1 << 20)  # Clear L bit
      inst & MASK32
    end

    # =====================================================================
    # ARM1 CPU Class
    # =====================================================================

    class ARM1
      # Creates a new ARM1 simulator with the given memory size (in bytes).
      #
      # On power-on, the ARM1 enters Supervisor mode with IRQs and FIQs
      # disabled, and begins executing from address 0x00000000.
      def initialize(memory_size = 1024 * 1024)
        memory_size = 1024 * 1024 if memory_size <= 0
        @memory = Array.new(memory_size, 0)
        @regs = Array.new(27, 0)
        @halted = false
        reset
      end

      # Restores the CPU to its power-on state:
      #   - Supervisor mode (SVC)
      #   - IRQs and FIQs disabled
      #   - PC = 0
      #   - All flags cleared
      def reset
        @regs.fill(0)
        @regs[15] = (FLAG_I | FLAG_F | MODE_SVC) & MASK32
        @halted = false
      end

      # -------------------------------------------------------------------
      # Register access
      # -------------------------------------------------------------------

      # Reads a register (R0-R15), respecting mode banking.
      def read_register(index)
        @regs[physical_reg(index)] & MASK32
      end

      # Writes a register (R0-R15), respecting mode banking.
      def write_register(index, value)
        @regs[physical_reg(index)] = value & MASK32
      end

      # Returns the current program counter (26-bit address).
      def pc
        @regs[15] & PC_MASK
      end

      # Sets the program counter portion of R15 without changing flags/mode.
      def set_pc(addr)
        @regs[15] = ((@regs[15] & ~PC_MASK) | (addr & PC_MASK)) & MASK32
      end

      # Returns the current condition flags as a Flags struct.
      def flags
        r15 = @regs[15]
        Flags.new(
          n: (r15 & FLAG_N) != 0,
          z: (r15 & FLAG_Z) != 0,
          c: (r15 & FLAG_C) != 0,
          v: (r15 & FLAG_V) != 0
        )
      end

      # Updates the condition flags in R15.
      def set_flags(f)
        r15 = @regs[15] & ~(FLAG_N | FLAG_Z | FLAG_C | FLAG_V) & MASK32
        r15 |= FLAG_N if f.n
        r15 |= FLAG_Z if f.z
        r15 |= FLAG_C if f.c
        r15 |= FLAG_V if f.v
        @regs[15] = r15 & MASK32
      end

      # Returns the current processor mode (0=USR, 1=FIQ, 2=IRQ, 3=SVC).
      def mode
        @regs[15] & MODE_MASK
      end

      # Returns true if the CPU has been halted.
      def halted?
        @halted
      end

      # -------------------------------------------------------------------
      # Memory access — byte-addressable, little-endian
      # -------------------------------------------------------------------

      # Reads a 32-bit word from memory (little-endian).
      def read_word(addr)
        addr &= PC_MASK
        a = addr & ~3  # Word-align
        return 0 if a + 3 >= @memory.length

        @memory[a] |
          (@memory[a + 1] << 8) |
          (@memory[a + 2] << 16) |
          (@memory[a + 3] << 24)
      end

      # Writes a 32-bit word to memory (little-endian).
      def write_word(addr, value)
        addr &= PC_MASK
        a = addr & ~3
        return if a + 3 >= @memory.length

        value &= MASK32
        @memory[a]     = value & 0xFF
        @memory[a + 1] = (value >> 8) & 0xFF
        @memory[a + 2] = (value >> 16) & 0xFF
        @memory[a + 3] = (value >> 24) & 0xFF
      end

      # Reads a single byte from memory.
      def read_byte(addr)
        addr &= PC_MASK
        return 0 if addr >= @memory.length

        @memory[addr]
      end

      # Writes a single byte to memory.
      def write_byte(addr, value)
        addr &= PC_MASK
        return if addr >= @memory.length

        @memory[addr] = value & 0xFF
      end

      # Returns a reference to the raw memory array.
      def memory
        @memory
      end

      # Loads machine code bytes into memory at the given start address.
      def load_program(code, start_addr = 0)
        code.each_with_index do |b, i|
          addr = start_addr + i
          @memory[addr] = b & 0xFF if addr < @memory.length
        end
      end

      # -------------------------------------------------------------------
      # Execution
      # -------------------------------------------------------------------

      # Executes one instruction and returns a Trace of what happened.
      def step
        current_pc = pc
        regs_before = Array.new(16) { |i| read_register(i) }
        flags_before = flags

        # Fetch
        instruction = read_word(current_pc)

        # Decode
        decoded = Arm1Simulator.decode(instruction)

        # Evaluate condition
        cond_met = Arm1Simulator.evaluate_condition(decoded.cond, flags_before)

        trace = Trace.new(
          address: current_pc,
          raw: instruction,
          mnemonic: Arm1Simulator.disassemble(decoded),
          condition: COND_NAMES.fetch(decoded.cond, "??"),
          condition_met: cond_met,
          regs_before: regs_before,
          flags_before: flags_before,
          memory_reads: [],
          memory_writes: []
        )

        # Advance PC (default: next instruction)
        set_pc(current_pc + 4)

        if cond_met
          case decoded.type
          when INST_DATA_PROCESSING
            execute_data_processing(decoded, trace)
          when INST_LOAD_STORE
            execute_load_store(decoded, trace)
          when INST_BLOCK_TRANSFER
            execute_block_transfer(decoded, trace)
          when INST_BRANCH
            execute_branch(decoded, trace)
          when INST_SWI
            execute_swi(decoded, trace)
          when INST_COPROCESSOR, INST_UNDEFINED
            trap_undefined(current_pc)
          end
        end

        # Capture state after execution
        trace.regs_after = Array.new(16) { |i| read_register(i) }
        trace.flags_after = flags

        trace
      end

      # Runs instructions until halted or max_steps reached.
      def run(max_steps)
        traces = []
        max_steps.times do
          break if @halted

          traces << step
        end
        traces
      end

      private

      # Maps a logical register index (0-15) to a physical register
      # index (0-26) based on the current processor mode.
      def physical_reg(index)
        m = mode
        case
        when m == MODE_FIQ && index >= 8 && index <= 14
          16 + (index - 8)
        when m == MODE_IRQ && index >= 13 && index <= 14
          23 + (index - 13)
        when m == MODE_SVC && index >= 13 && index <= 14
          25 + (index - 13)
        else
          index
        end
      end

      # Reads a register as it would appear during instruction execution.
      # For R15, returns PC + 8 (pipeline effect). Since we already
      # advanced PC by 4 in step(), we add 4 more.
      def read_reg_for_exec(index)
        if index == 15
          (@regs[15] + 4) & MASK32
        else
          read_register(index)
        end
      end

      # -------------------------------------------------------------------
      # Data Processing execution
      # -------------------------------------------------------------------

      def execute_data_processing(d, trace)
        # Get first operand (Rn)
        a = if d.opcode != OP_MOV && d.opcode != OP_MVN
              read_reg_for_exec(d.rn)
            else
              0
            end

        # Get second operand through barrel shifter
        f = flags
        if d.immediate
          b, shifter_carry = Arm1Simulator.decode_immediate(d.imm8, d.rotate)
          shifter_carry = f.c if d.rotate == 0
        else
          rm_val = read_reg_for_exec(d.rm)
          shift_amount = if d.shift_by_reg
                           read_reg_for_exec(d.rs) & 0xFF
                         else
                           d.shift_imm || 0
                         end
          b, shifter_carry = Arm1Simulator.barrel_shift(
            rm_val, d.shift_type, shift_amount, f.c, d.shift_by_reg
          )
        end

        # Execute ALU operation
        result = Arm1Simulator.alu_execute(d.opcode, a, b, f.c, shifter_carry, f.v)

        # Write result to Rd (unless test-only operation)
        if result.write_result
          if d.rd == 15
            if d.s
              # MOVS PC, LR — restore PC and flags
              @regs[15] = result.result & MASK32
            else
              set_pc(result.result & PC_MASK)
            end
          else
            write_register(d.rd, result.result)
          end
        end

        # Update flags if S bit set (and Rd is not R15)
        if d.s && d.rd != 15
          set_flags(Flags.new(n: result.n, z: result.z, c: result.c, v: result.v))
        end
        # Test-only ops always update flags
        if Arm1Simulator.test_op?(d.opcode)
          set_flags(Flags.new(n: result.n, z: result.z, c: result.c, v: result.v))
        end
      end

      # -------------------------------------------------------------------
      # Load/Store execution
      # -------------------------------------------------------------------

      def execute_load_store(d, trace)
        # Compute offset
        if d.immediate
          rm_val = read_reg_for_exec(d.rm)
          if (d.shift_imm || 0) != 0
            rm_val, _ = Arm1Simulator.barrel_shift(rm_val, d.shift_type, d.shift_imm, flags.c, false)
          end
          offset = rm_val
        else
          offset = d.offset12 || 0
        end

        # Base address
        base = read_reg_for_exec(d.rn)

        # Compute effective address
        addr = d.up ? (base + offset) & MASK32 : (base - offset) & MASK32

        # Pre-indexed vs post-indexed
        transfer_addr = d.pre_index ? addr : base

        if d.load
          if d.byte
            value = read_byte(transfer_addr)
          else
            value = read_word(transfer_addr)
            # ARM1 quirk: unaligned word loads rotate the data
            rotation = (transfer_addr & 3) * 8
            if rotation != 0
              value = ((value >> rotation) | (value << (32 - rotation))) & MASK32
            end
          end
          trace.memory_reads << MemoryAccess.new(address: transfer_addr, value: value)

          if d.rd == 15
            @regs[15] = value & MASK32
          else
            write_register(d.rd, value)
          end
        else
          value = read_reg_for_exec(d.rd)
          if d.byte
            write_byte(transfer_addr, value & 0xFF)
          else
            write_word(transfer_addr, value)
          end
          trace.memory_writes << MemoryAccess.new(address: transfer_addr, value: value)
        end

        # Write-back
        if d.write_back || !d.pre_index
          write_register(d.rn, addr) if d.rn != 15
        end
      end

      # -------------------------------------------------------------------
      # Block Transfer execution (LDM/STM)
      # -------------------------------------------------------------------

      def execute_block_transfer(d, trace)
        base = read_register(d.rn)
        reg_list = d.register_list || 0

        # Count registers in the list
        count = 0
        16.times { |i| count += 1 if ((reg_list >> i) & 1) == 1 }
        return if count == 0

        # Calculate start address
        start_addr = case [d.pre_index, d.up]
                     when [false, true]  then base                          # IA
                     when [true, true]   then base + 4                      # IB
                     when [false, false] then base - (count * 4) + 4        # DA
                     when [true, false]  then base - (count * 4)            # DB
                     end
        start_addr &= MASK32

        addr = start_addr
        16.times do |i|
          next if ((reg_list >> i) & 1) == 0

          if d.load
            value = read_word(addr)
            trace.memory_reads << MemoryAccess.new(address: addr, value: value)
            if i == 15
              @regs[15] = value & MASK32
            else
              write_register(i, value)
            end
          else
            value = if i == 15
                      (@regs[15] + 4) & MASK32  # PC + 8 (we already added 4)
                    else
                      read_register(i)
                    end
            write_word(addr, value)
            trace.memory_writes << MemoryAccess.new(address: addr, value: value)
          end
          addr = (addr + 4) & MASK32
        end

        # Write-back
        if d.write_back
          new_base = if d.up
                       (base + (count * 4)) & MASK32
                     else
                       (base - (count * 4)) & MASK32
                     end
          write_register(d.rn, new_base)
        end
      end

      # -------------------------------------------------------------------
      # Branch execution
      # -------------------------------------------------------------------

      def execute_branch(d, trace)
        # The branch offset is relative to PC + 8 from the original instruction.
        # Since we already did PC += 4, we need PC + 4 more.
        branch_base = (pc + 4) & MASK32

        if d.link
          # BL: save return address in R14 (LR)
          return_addr = @regs[15] & MASK32
          write_register(14, return_addr)
        end

        # Compute target address
        target = (branch_base + d.branch_offset) & MASK32
        set_pc(target & PC_MASK)
      end

      # -------------------------------------------------------------------
      # SWI execution
      # -------------------------------------------------------------------

      def execute_swi(d, trace)
        if d.swi_comment == HALT_SWI
          @halted = true
          return
        end

        # Real SWI: enter Supervisor mode
        @regs[25] = @regs[15] & MASK32
        @regs[26] = @regs[15] & MASK32

        r15 = @regs[15]
        r15 = (r15 & ~MODE_MASK) | MODE_SVC
        r15 |= FLAG_I
        @regs[15] = r15 & MASK32

        set_pc(0x08)
      end

      # -------------------------------------------------------------------
      # Exception handling
      # -------------------------------------------------------------------

      def trap_undefined(_instr_addr)
        @regs[26] = @regs[15] & MASK32

        r15 = @regs[15]
        r15 = (r15 & ~MODE_MASK) | MODE_SVC
        r15 |= FLAG_I
        @regs[15] = r15 & MASK32

        set_pc(0x04)
      end
    end
  end
end
