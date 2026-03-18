# frozen_string_literal: true

# ==========================================================================
# OpCode -- The Complete Instruction Set
# ==========================================================================
#
# Each opcode is an integer constant, grouped by category:
#
#   0x0_ = stack operations
#   0x1_ = variable operations
#   0x2_ = arithmetic
#   0x3_ = comparison
#   0x4_ = control flow
#   0x5_ = function operations
#   0x6_ = I/O
#   0xF_ = VM control
#
# This mirrors how the JVM organizes its bytecode -- all "load"
# instructions in one numeric range, all "store" in another.
# ==========================================================================

module CodingAdventures
  module VirtualMachine
    module OpCode
      # -- Stack Operations (0x0_) --
      LOAD_CONST    = 0x01  # Push a constant from the constants pool
      POP           = 0x02  # Discard the top of stack
      DUP           = 0x03  # Duplicate the top of stack

      # -- Variable Operations (0x1_) --
      STORE_NAME    = 0x10  # Pop and store in a named variable
      LOAD_NAME     = 0x11  # Push a named variable's value
      STORE_LOCAL   = 0x12  # Pop and store in a local slot
      LOAD_LOCAL    = 0x13  # Push a local slot's value

      # -- Arithmetic (0x2_) --
      ADD           = 0x20  # Pop two, push sum
      SUB           = 0x21  # Pop two, push difference (a - b)
      MUL           = 0x22  # Pop two, push product
      DIV           = 0x23  # Pop two, push quotient (integer division)

      # -- Comparison (0x3_) --
      CMP_EQ        = 0x30  # Pop two, push 1 if equal, 0 otherwise
      CMP_LT        = 0x31  # Pop two, push 1 if a < b, 0 otherwise
      CMP_GT        = 0x32  # Pop two, push 1 if a > b, 0 otherwise

      # -- Control Flow (0x4_) --
      JUMP          = 0x40  # Unconditional jump to operand
      JUMP_IF_FALSE = 0x41  # Pop; jump if falsy (0, nil, "")
      JUMP_IF_TRUE  = 0x42  # Pop; jump if truthy

      # -- Functions (0x5_) --
      CALL          = 0x50  # Call a function
      RETURN        = 0x51  # Return from a function

      # -- I/O (0x6_) --
      PRINT         = 0x60  # Pop and print

      # -- VM Control (0xF_) --
      HALT          = 0xFF  # Stop execution

      # Map from opcode integer to name string for descriptions.
      NAMES = {
        LOAD_CONST => "LOAD_CONST", POP => "POP", DUP => "DUP",
        STORE_NAME => "STORE_NAME", LOAD_NAME => "LOAD_NAME",
        STORE_LOCAL => "STORE_LOCAL", LOAD_LOCAL => "LOAD_LOCAL",
        ADD => "ADD", SUB => "SUB", MUL => "MUL", DIV => "DIV",
        CMP_EQ => "CMP_EQ", CMP_LT => "CMP_LT", CMP_GT => "CMP_GT",
        JUMP => "JUMP", JUMP_IF_FALSE => "JUMP_IF_FALSE",
        JUMP_IF_TRUE => "JUMP_IF_TRUE",
        CALL => "CALL", RETURN => "RETURN",
        PRINT => "PRINT", HALT => "HALT"
      }.freeze
    end
  end
end
