# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Opcodes and Instructions -- the vocabulary of GPU core programs.
# ---------------------------------------------------------------------------
#
# === What is an Opcode? ===
#
# An opcode (operation code) is a number or name that tells the processor what
# to do. It's like a verb in a sentence:
#
#     English:  "Add the first two numbers and store in the third"
#     Assembly: FADD R2, R0, R1
#
# The opcode is FADD. The registers R0, R1, R2 are the operands.
#
# === Instruction Representation ===
#
# Real GPU hardware represents instructions as binary words (32 or 64 bits of
# 1s and 0s packed together). But at this layer -- the processing element
# simulator -- we use a structured Ruby object instead:
#
#     Binary (real hardware): 01001000_00000010_00000000_00000001
#     Our representation:     Instruction.new(:fadd, rd: 2, rs1: 0, rs2: 1)
#
# Why? Because binary encoding is the job of the *assembler* layer above us.
# The processing element receives already-decoded instructions from the
# instruction cache. We're simulating what happens *after* decode.
#
# === The Instruction Set ===
#
# Our GenericISA has 16 opcodes organized into four categories:
#
#     Arithmetic:  FADD, FSUB, FMUL, FFMA, FNEG, FABS  (6 opcodes)
#     Memory:      LOAD, STORE                           (2 opcodes)
#     Data move:   MOV, LIMM                             (2 opcodes)
#     Control:     BEQ, BLT, BNE, JMP, NOP, HALT         (6 opcodes)
#
# This is deliberately minimal. Real ISAs have hundreds of opcodes, but these
# 16 are enough to write any floating-point program (they're Turing-complete
# when combined with branches and memory).
#
# === Helper Constructors ===
#
# Writing programs as raw Instruction.new(...) calls is verbose. The helper
# module methods (fadd, fmul, ffma, load, store, limm, halt, etc.) make
# programs readable:
#
#     # Without helpers (verbose):
#     program = [
#       Instruction.new(:limm, rd: 0, immediate: 2.0),
#       Instruction.new(:limm, rd: 1, immediate: 3.0),
#       Instruction.new(:fmul, rd: 2, rs1: 0, rs2: 1),
#       Instruction.new(:halt),
#     ]
#
#     # With helpers (clean):
#     program = [
#       GpuCore.limm(0, 2.0),
#       GpuCore.limm(1, 3.0),
#       GpuCore.fmul(2, 0, 1),
#       GpuCore.halt,
#     ]

module CodingAdventures
  module GpuCore
    # The set of operations a GPU core can perform.
    #
    # We use Ruby symbols instead of an Enum class. Symbols are the idiomatic
    # Ruby equivalent of Python's Enum -- they're immutable, interned strings
    # that are perfect for fixed sets of identifiers.
    #
    # Organized by category:
    #
    # Floating-point arithmetic (uses fp-arithmetic package):
    #     :fadd  -- add two registers
    #     :fsub  -- subtract two registers
    #     :fmul  -- multiply two registers
    #     :ffma  -- fused multiply-add (three source registers)
    #     :fneg  -- negate a register
    #     :fabs  -- absolute value of a register
    #
    # Memory operations:
    #     :load  -- load float from memory into register
    #     :store -- store register value to memory
    #
    # Data movement:
    #     :mov   -- copy one register to another
    #     :limm  -- load an immediate (literal) float value
    #
    # Control flow:
    #     :beq   -- branch if equal
    #     :blt   -- branch if less than
    #     :bne   -- branch if not equal
    #     :jmp   -- unconditional jump
    #     :nop   -- no operation
    #     :halt  -- stop execution
    ALL_OPCODES = %i[
      fadd fsub fmul ffma fneg fabs
      load store
      mov limm
      beq blt bne jmp nop halt
    ].freeze

    # Instruction -- a single GPU core instruction.
    #
    # This is a structured representation of an instruction, not a binary
    # encoding. It contains all the information needed to execute the
    # instruction: the opcode and up to four operands.
    #
    # We use Data.define (Ruby 3.2+) to create an immutable value object,
    # mirroring Python's frozen dataclass. Once created, fields cannot be
    # changed -- instructions are constants in the program.
    #
    # Fields:
    #     opcode:    What operation to perform (a symbol from ALL_OPCODES).
    #     rd:        Destination register index (0-255).
    #     rs1:       First source register index (0-255).
    #     rs2:       Second source register index (0-255).
    #     rs3:       Third source register (used only by FFMA).
    #     immediate: A literal float value (used by LIMM, branch offsets,
    #                memory offsets). For branches, this is the number of
    #                instructions to skip (positive = forward, negative = back).
    Instruction = Data.define(:opcode, :rd, :rs1, :rs2, :rs3, :immediate) do
      # Pretty-print the instruction in assembly-like syntax.
      #
      # This makes programs human-readable when printed, matching the style
      # you'd see in a GPU disassembler or debugger.
      def to_s
        op = opcode.to_s.upcase
        case opcode
        when :fadd, :fsub, :fmul
          "#{op} R#{rd}, R#{rs1}, R#{rs2}"
        when :ffma
          "#{op} R#{rd}, R#{rs1}, R#{rs2}, R#{rs3}"
        when :fneg, :fabs
          "#{op} R#{rd}, R#{rs1}"
        when :load
          "#{op} R#{rd}, [R#{rs1}+#{immediate}]"
        when :store
          "#{op} [R#{rs1}+#{immediate}], R#{rs2}"
        when :mov
          "#{op} R#{rd}, R#{rs1}"
        when :limm
          "#{op} R#{rd}, #{immediate}"
        when :beq, :blt, :bne
          sign = immediate >= 0 ? "+" : ""
          "#{op} R#{rs1}, R#{rs2}, #{sign}#{immediate.to_i}"
        when :jmp
          "#{op} #{immediate.to_i}"
        when :nop
          "NOP"
        when :halt
          "HALT"
        else
          "#{op} rd=#{rd} rs1=#{rs1} rs2=#{rs2}"
        end
      end

      # Ruby convention: inspect delegates to to_s for readable output.
      def inspect
        to_s
      end
    end

    # -----------------------------------------------------------------------
    # Helper constructors -- make programs readable
    # -----------------------------------------------------------------------
    #
    # These module methods create Instruction objects with sensible defaults.
    # They're the primary way to write GPU programs in this simulator.

    # FADD Rd, Rs1, Rs2 -- floating-point addition: Rd = Rs1 + Rs2.
    def self.fadd(rd, rs1, rs2)
      Instruction.new(opcode: :fadd, rd: rd, rs1: rs1, rs2: rs2, rs3: 0, immediate: 0.0)
    end

    # FSUB Rd, Rs1, Rs2 -- floating-point subtraction: Rd = Rs1 - Rs2.
    def self.fsub(rd, rs1, rs2)
      Instruction.new(opcode: :fsub, rd: rd, rs1: rs1, rs2: rs2, rs3: 0, immediate: 0.0)
    end

    # FMUL Rd, Rs1, Rs2 -- floating-point multiplication: Rd = Rs1 * Rs2.
    def self.fmul(rd, rs1, rs2)
      Instruction.new(opcode: :fmul, rd: rd, rs1: rs1, rs2: rs2, rs3: 0, immediate: 0.0)
    end

    # FFMA Rd, Rs1, Rs2, Rs3 -- fused multiply-add: Rd = Rs1 * Rs2 + Rs3.
    def self.ffma(rd, rs1, rs2, rs3)
      Instruction.new(opcode: :ffma, rd: rd, rs1: rs1, rs2: rs2, rs3: rs3, immediate: 0.0)
    end

    # FNEG Rd, Rs1 -- negate: Rd = -Rs1.
    def self.fneg(rd, rs1)
      Instruction.new(opcode: :fneg, rd: rd, rs1: rs1, rs2: 0, rs3: 0, immediate: 0.0)
    end

    # FABS Rd, Rs1 -- absolute value: Rd = |Rs1|.
    def self.fabs(rd, rs1)
      Instruction.new(opcode: :fabs, rd: rd, rs1: rs1, rs2: 0, rs3: 0, immediate: 0.0)
    end

    # LOAD Rd, [Rs1+offset] -- load float from memory into register.
    def self.load(rd, rs1, offset = 0.0)
      Instruction.new(opcode: :load, rd: rd, rs1: rs1, rs2: 0, rs3: 0, immediate: offset)
    end

    # STORE [Rs1+offset], Rs2 -- store register value to memory.
    def self.store(rs1, rs2, offset = 0.0)
      Instruction.new(opcode: :store, rd: 0, rs1: rs1, rs2: rs2, rs3: 0, immediate: offset)
    end

    # MOV Rd, Rs1 -- copy register: Rd = Rs1.
    def self.mov(rd, rs1)
      Instruction.new(opcode: :mov, rd: rd, rs1: rs1, rs2: 0, rs3: 0, immediate: 0.0)
    end

    # LIMM Rd, value -- load immediate float: Rd = value.
    def self.limm(rd, value)
      Instruction.new(opcode: :limm, rd: rd, rs1: 0, rs2: 0, rs3: 0, immediate: value)
    end

    # BEQ Rs1, Rs2, offset -- branch if equal.
    def self.beq(rs1, rs2, offset)
      Instruction.new(opcode: :beq, rd: 0, rs1: rs1, rs2: rs2, rs3: 0, immediate: offset.to_f)
    end

    # BLT Rs1, Rs2, offset -- branch if less than.
    def self.blt(rs1, rs2, offset)
      Instruction.new(opcode: :blt, rd: 0, rs1: rs1, rs2: rs2, rs3: 0, immediate: offset.to_f)
    end

    # BNE Rs1, Rs2, offset -- branch if not equal.
    def self.bne(rs1, rs2, offset)
      Instruction.new(opcode: :bne, rd: 0, rs1: rs1, rs2: rs2, rs3: 0, immediate: offset.to_f)
    end

    # JMP target -- unconditional jump to absolute address.
    def self.jmp(target)
      Instruction.new(opcode: :jmp, rd: 0, rs1: 0, rs2: 0, rs3: 0, immediate: target.to_f)
    end

    # NOP -- no operation, advance program counter.
    def self.nop
      Instruction.new(opcode: :nop, rd: 0, rs1: 0, rs2: 0, rs3: 0, immediate: 0.0)
    end

    # HALT -- stop execution.
    def self.halt
      Instruction.new(opcode: :halt, rd: 0, rs1: 0, rs2: 0, rs3: 0, immediate: 0.0)
    end
  end
end
