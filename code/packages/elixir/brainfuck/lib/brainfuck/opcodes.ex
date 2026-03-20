defmodule CodingAdventures.Brainfuck.Opcodes do
  @moduledoc """
  Brainfuck Opcodes — The Simplest Instruction Set Imaginable.

  ## From 8 Characters to 9 Opcodes

  Brainfuck has exactly 8 commands. We map each to a numeric opcode, plus
  HALT to mark the end of the program. These opcodes are registered with
  the GenericVM via `register_opcode/3`.

  ## Why Numeric Opcodes Instead of Characters?

  Because the GenericVM dispatches on integers — it is a *bytecode*
  interpreter, not a character interpreter. This also means the same
  GenericVM that runs Starlark's 0x01-0xFF opcodes can run Brainfuck's
  0x01-0x08 opcodes. Different opcode *numbers*, different *handlers*,
  same execution engine.

  ## Opcode Table

      Opcode       Hex    BF    Description
      ──────────────────────────────────────────────
      RIGHT        0x01   >     Move data pointer right
      LEFT         0x02   <     Move data pointer left
      INC          0x03   +     Increment current cell
      DEC          0x04   -     Decrement current cell
      OUTPUT       0x05   .     Print cell as ASCII
      INPUT        0x06   ,     Read byte into cell
      LOOP_START   0x07   [     Jump forward if cell == 0
      LOOP_END     0x08   ]     Jump backward if cell != 0
      HALT         0xFF   —     Stop execution

  Note that Brainfuck opcodes have **no stack effect**. Unlike a stack-based
  language where you push, push, add, pop, Brainfuck operates entirely on
  the tape. The GenericVM's operand stack goes unused — but it is still
  there, available if a future language needs it.

  ## Character-to-Opcode Mapping

  The `char_to_op/0` function returns a map from Brainfuck characters to
  their opcode numbers. Characters not in this map are ignored during
  translation (they are comments — Brainfuck's only comment syntax).
  """

  # =========================================================================
  # Opcode constants
  # =========================================================================
  # Each constant is a module attribute so it can be used in pattern matches
  # and guard clauses. The public functions expose them for other modules.

  @right 0x01
  @left 0x02
  @inc 0x03
  @dec 0x04
  @output 0x05
  @input 0x06
  @loop_start 0x07
  @loop_end 0x08
  @halt 0xFF

  @doc "`>` — Move the data pointer one cell to the right."
  def right, do: @right

  @doc "`<` — Move the data pointer one cell to the left."
  def left, do: @left

  @doc "`+` — Increment the byte at the data pointer (wraps 255 -> 0)."
  def inc, do: @inc

  @doc "`-` — Decrement the byte at the data pointer (wraps 0 -> 255)."
  def dec, do: @dec

  @doc "`.` — Output the byte at the data pointer as an ASCII character."
  def output_op, do: @output

  @doc "`,` — Read one byte of input into the current cell."
  def input_op, do: @input

  @doc "`[` — If current cell is zero, jump forward past matching `]`."
  def loop_start, do: @loop_start

  @doc "`]` — If current cell is nonzero, jump backward to matching `[`."
  def loop_end, do: @loop_end

  @doc "Stop execution."
  def halt, do: @halt

  # =========================================================================
  # Character-to-opcode mapping
  # =========================================================================

  @char_to_op %{
    ">" => @right,
    "<" => @left,
    "+" => @inc,
    "-" => @dec,
    "." => @output,
    "," => @input,
    "[" => @loop_start,
    "]" => @loop_end
  }

  @doc """
  Map from Brainfuck source characters to opcode integers.

  Characters not in this map are ignored during translation — they are
  treated as comments. This is Brainfuck's only comment syntax: anything
  that is not one of `><+-.,[]` is a comment.

  ## Example

      iex> CodingAdventures.Brainfuck.Opcodes.char_to_op()["+"]
      0x03
  """
  def char_to_op, do: @char_to_op
end
