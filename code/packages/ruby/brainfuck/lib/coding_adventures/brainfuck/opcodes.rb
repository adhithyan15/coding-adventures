# frozen_string_literal: true

# ==========================================================================
# Brainfuck Opcodes — The Simplest Instruction Set
# ==========================================================================
#
# Brainfuck has 8 commands. We map each to a numeric opcode, plus HALT to
# mark the end of the program. These opcodes are registered with the
# GenericVM via register_opcode().
#
# Why numeric opcodes instead of characters? Because the GenericVM dispatches
# on integers — it's a *bytecode* interpreter, not a character interpreter.
# This also means the same GenericVM that runs Starlark's 0x01-0xFF opcodes
# can run Brainfuck's 0x01-0x08 opcodes. Different opcode *numbers*,
# different *handlers*, same execution engine.
#
# ==========================================================================
# Opcode Table
# ==========================================================================
#
#   Opcode       Hex    BF    Description
#   ─────────────────────────────────────────────────────────────
#   RIGHT        0x01   >     Move data pointer right
#   LEFT         0x02   <     Move data pointer left
#   INC          0x03   +     Increment current cell
#   DEC          0x04   -     Decrement current cell
#   OUTPUT       0x05   .     Print cell as ASCII
#   INPUT        0x06   ,     Read byte into cell
#   LOOP_START   0x07   [     Jump forward if cell == 0
#   LOOP_END     0x08   ]     Jump backward if cell != 0
#   HALT         0xFF   —     Stop execution
#
# Note that Brainfuck opcodes have no stack effect. Unlike Starlark's
# stack-based arithmetic, Brainfuck operates entirely on the tape.
# The GenericVM's operand stack goes unused.
# ==========================================================================

module CodingAdventures
  module Brainfuck
    # Brainfuck opcode constants.
    #
    # Each constant corresponds to one of the 8 Brainfuck commands,
    # plus HALT for end-of-program.
    module Op
      # -- Pointer movement --------------------------------------------------
      RIGHT      = 0x01  # >  Move the data pointer one cell to the right
      LEFT       = 0x02  # <  Move the data pointer one cell to the left

      # -- Cell modification -------------------------------------------------
      INC        = 0x03  # +  Increment the byte at the data pointer (wraps 255 → 0)
      DEC        = 0x04  # -  Decrement the byte at the data pointer (wraps 0 → 255)

      # -- I/O ---------------------------------------------------------------
      OUTPUT     = 0x05  # .  Output the byte at the data pointer as ASCII
      INPUT      = 0x06  # ,  Read one byte of input into the current cell

      # -- Control flow ------------------------------------------------------
      LOOP_START = 0x07  # [  If cell == 0, jump forward past matching ]
      LOOP_END   = 0x08  # ]  If cell != 0, jump backward to matching [

      # -- VM control --------------------------------------------------------
      HALT       = 0xFF  # End of program — stop the VM
    end

    # Maps each Brainfuck character to its opcode. Characters not in this
    # hash are ignored (they're comments).
    CHAR_TO_OP = {
      ">" => Op::RIGHT,
      "<" => Op::LEFT,
      "+" => Op::INC,
      "-" => Op::DEC,
      "." => Op::OUTPUT,
      "," => Op::INPUT,
      "[" => Op::LOOP_START,
      "]" => Op::LOOP_END
    }.freeze
  end
end
