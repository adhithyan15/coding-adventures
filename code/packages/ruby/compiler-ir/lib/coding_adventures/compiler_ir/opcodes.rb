# frozen_string_literal: true

# ==========================================================================
# IrOp — Opcode Enumeration for the General-Purpose IR
# ==========================================================================
#
# This module defines every opcode in the intermediate representation.
# Opcodes are plain integer constants grouped by category:
#
#   Constants:    LOAD_IMM, LOAD_ADDR
#   Memory:       LOAD_BYTE, STORE_BYTE, LOAD_WORD, STORE_WORD
#   Arithmetic:   ADD, ADD_IMM, SUB, AND, AND_IMM
#   Comparison:   CMP_EQ, CMP_NE, CMP_LT, CMP_GT
#   Control Flow: LABEL, JUMP, BRANCH_Z, BRANCH_NZ, CALL, RET
#   System:       SYSCALL, HALT
#   Meta:         NOP, COMMENT
#
# Design rules (the same rules as the Go port):
#   1. Existing opcode integer values never change — only new ones are appended.
#   2. A new opcode is added only when a frontend needs it AND it cannot
#      be efficiently expressed as a sequence of existing opcodes.
#   3. All frontends and backends remain forward-compatible.
#
# The opcode values are assigned sequentially starting at 0, matching the
# Go iota order exactly so that text files produced by one implementation
# are readable by the other.
# ==========================================================================

module CodingAdventures
  module CompilerIr
    module IrOp
      # ── Constants ──────────────────────────────────────────────────────────
      # Load an immediate integer value into a register.
      #   LOAD_IMM  v0, 42    →  v0 = 42
      LOAD_IMM = 0

      # Load the address of a data label into a register.
      #   LOAD_ADDR v0, tape  →  v0 = &tape
      LOAD_ADDR = 1

      # ── Memory ────────────────────────────────────────────────────────────
      # Load a byte from memory: dst = mem[base + offset] (zero-extended).
      #   LOAD_BYTE v2, v0, v1  →  v2 = mem[v0 + v1] & 0xFF
      LOAD_BYTE = 2

      # Store a byte to memory: mem[base + offset] = src & 0xFF.
      #   STORE_BYTE v2, v0, v1  →  mem[v0 + v1] = v2 & 0xFF
      STORE_BYTE = 3

      # Load a machine word from memory: dst = *(word*)(base + offset).
      #   LOAD_WORD v2, v0, v1  →  v2 = *(int*)(v0 + v1)
      LOAD_WORD = 4

      # Store a machine word to memory: *(word*)(base + offset) = src.
      #   STORE_WORD v2, v0, v1  →  *(int*)(v0 + v1) = v2
      STORE_WORD = 5

      # ── Arithmetic ────────────────────────────────────────────────────────
      # Register-register addition: dst = lhs + rhs.
      #   ADD v3, v1, v2  →  v3 = v1 + v2
      ADD = 6

      # Register-immediate addition: dst = src + immediate.
      #   ADD_IMM v1, v1, 1  →  v1 = v1 + 1
      ADD_IMM = 7

      # Register-register subtraction: dst = lhs - rhs.
      #   SUB v3, v1, v2  →  v3 = v1 - v2
      SUB = 8

      # Register-register bitwise AND: dst = lhs & rhs.
      #   AND v3, v1, v2  →  v3 = v1 & v2
      AND = 9

      # Register-immediate bitwise AND: dst = src & immediate.
      #   AND_IMM v2, v2, 255  →  v2 = v2 & 0xFF
      AND_IMM = 10

      # ── Comparison ────────────────────────────────────────────────────────
      # Set dst = 1 if lhs == rhs, else 0.
      #   CMP_EQ v4, v1, v2  →  v4 = (v1 == v2) ? 1 : 0
      CMP_EQ = 11

      # Set dst = 1 if lhs != rhs, else 0.
      #   CMP_NE v4, v1, v2  →  v4 = (v1 != v2) ? 1 : 0
      CMP_NE = 12

      # Set dst = 1 if lhs < rhs (signed), else 0.
      #   CMP_LT v4, v1, v2  →  v4 = (v1 < v2) ? 1 : 0
      CMP_LT = 13

      # Set dst = 1 if lhs > rhs (signed), else 0.
      #   CMP_GT v4, v1, v2  →  v4 = (v1 > v2) ? 1 : 0
      CMP_GT = 14

      # ── Control Flow ──────────────────────────────────────────────────────
      # Define a label at this point in the instruction stream.
      # Labels produce no machine code — they just record an address.
      #   LABEL loop_start
      LABEL = 15

      # Unconditional jump to a label.
      #   JUMP loop_start  →  PC = &loop_start
      JUMP = 16

      # Conditional branch: jump to label if register == 0.
      #   BRANCH_Z v2, loop_end  →  if v2 == 0 then PC = &loop_end
      BRANCH_Z = 17

      # Conditional branch: jump to label if register != 0.
      #   BRANCH_NZ v2, loop_end  →  if v2 != 0 then PC = &loop_end
      BRANCH_NZ = 18

      # Call a subroutine at the given label. Pushes return address.
      #   CALL my_func
      CALL = 19

      # Return from a subroutine. Pops return address.
      #   RET
      RET = 20

      # ── System ────────────────────────────────────────────────────────────
      # Invoke a system call. The syscall number is an immediate operand.
      # Arguments and return values follow the platform's syscall ABI.
      #   SYSCALL 1  →  ecall with a7=1 (write)
      SYSCALL = 21

      # Halt execution. The program terminates.
      #   HALT  →  ecall with a7=10 (exit)
      HALT = 22

      # ── Meta ──────────────────────────────────────────────────────────────
      # No operation. Produces a single NOP instruction in the backend.
      #   NOP
      NOP = 23

      # A human-readable comment. Produces no machine code.
      # Useful for debugging IR output.
      #   COMMENT "load tape base address"
      COMMENT = 24

      # ── Name tables ───────────────────────────────────────────────────────
      # Maps each opcode integer to its canonical text name.
      # These names are used by the IR printer and parser for roundtrip fidelity.
      #
      # The canonical name is the "assembly language" mnemonic. These strings
      # appear verbatim in .ir text files and must never change for a given
      # opcode value.
      OP_NAMES = {
        LOAD_IMM => "LOAD_IMM",
        LOAD_ADDR => "LOAD_ADDR",
        LOAD_BYTE => "LOAD_BYTE",
        STORE_BYTE => "STORE_BYTE",
        LOAD_WORD => "LOAD_WORD",
        STORE_WORD => "STORE_WORD",
        ADD => "ADD",
        ADD_IMM => "ADD_IMM",
        SUB => "SUB",
        AND => "AND",
        AND_IMM => "AND_IMM",
        CMP_EQ => "CMP_EQ",
        CMP_NE => "CMP_NE",
        CMP_LT => "CMP_LT",
        CMP_GT => "CMP_GT",
        LABEL => "LABEL",
        JUMP => "JUMP",
        BRANCH_Z => "BRANCH_Z",
        BRANCH_NZ => "BRANCH_NZ",
        CALL => "CALL",
        RET => "RET",
        SYSCALL => "SYSCALL",
        HALT => "HALT",
        NOP => "NOP",
        COMMENT => "COMMENT"
      }.freeze

      # The reverse mapping: text name → opcode integer.
      # Built once from OP_NAMES at load time.
      NAME_TO_OP = OP_NAMES.invert.freeze

      # op_name(op) → String
      #
      # Returns the canonical text name for an opcode integer.
      # Returns "UNKNOWN" if the opcode is not recognised.
      #
      # Example:
      #   IrOp.op_name(IrOp::ADD_IMM)  #=> "ADD_IMM"
      def self.op_name(op)
        OP_NAMES.fetch(op, "UNKNOWN")
      end

      # parse_op(name) → Integer or nil
      #
      # Converts a text opcode name to its integer value.
      # Returns nil if the name is not recognised.
      # This is the inverse of op_name.
      #
      # Example:
      #   IrOp.parse_op("ADD_IMM")  #=> 7
      #   IrOp.parse_op("BOGUS")    #=> nil
      def self.parse_op(name)
        NAME_TO_OP[name]
      end
    end
  end
end
