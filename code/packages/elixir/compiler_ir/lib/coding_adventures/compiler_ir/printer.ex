defmodule CodingAdventures.CompilerIr.Printer do
  @moduledoc """
  IR Printer — `IrProgram` → human-readable text.

  The printer converts an `IrProgram` into its canonical text format.
  This format serves three purposes:

  1. **Debugging** — humans can read the IR to understand what the
     compiler produced.
  2. **Golden-file tests** — expected IR output is committed as `.ir`
     text files and compared against actual output.
  3. **Roundtrip** — `parse(print(program)) == program` is a testable
     invariant.

  ## Text format

      .version 1

      .data tape 30000 0

      .entry _start

      _start:
        LOAD_ADDR   v0, tape          ; #0
        LOAD_IMM    v1, 0             ; #1
        HALT                          ; #2

  ## Key rules

  - `.version N` is always the first non-comment line.
  - `.data` declarations come before `.entry`.
  - Labels are on their own unindented line with a trailing colon.
  - Instructions are indented with two spaces.
  - `; #N` comments show instruction IDs (informational only).
  - `COMMENT` instructions emit as `; <text>` (no ID).
  """

  alias CodingAdventures.CompilerIr.{IrProgram, IrInstruction, IrDataDecl, IrOp}
  alias CodingAdventures.CompilerIr.{IrRegister, IrImmediate, IrLabel}

  @doc """
  Convert an `IrProgram` to its canonical text representation.

  ## Examples

      iex> p = IrProgram.new("_start")
      iex> text = Printer.print(p)
      iex> String.starts_with?(text, ".version 1")
      true
  """
  @spec print(IrProgram.t()) :: String.t()
  def print(%IrProgram{} = program) do
    parts = [
      # Version directive — always first
      ".version #{program.version}\n",

      # Data declarations — one per .data line
      print_data(program.data),

      # Entry point directive
      "\n.entry #{program.entry_label}\n",

      # Instructions
      print_instructions(program.instructions)
    ]

    Enum.join(parts)
  end

  # ── Private helpers ──────────────────────────────────────────────────────────

  # Emit each .data declaration on its own line, preceded by a blank line.
  defp print_data([]), do: ""

  defp print_data(decls) do
    Enum.map_join(decls, "", fn %IrDataDecl{label: lbl, size: sz, init: init} ->
      "\n.data #{lbl} #{sz} #{init}\n"
    end)
  end

  # Emit all instructions.
  defp print_instructions(instructions) do
    Enum.map_join(instructions, "", &print_instruction/1)
  end

  # Label instructions get their own unindented line with a trailing colon.
  # They produce no machine code, so no ID comment.
  defp print_instruction(%IrInstruction{opcode: :label, operands: [operand]}) do
    "\n#{operand_to_string(operand)}:\n"
  end

  # COMMENT instructions emit as "; <text>" — no ID comment.
  defp print_instruction(%IrInstruction{opcode: :comment, operands: operands}) do
    text =
      case operands do
        [op | _] -> operand_to_string(op)
        [] -> ""
      end

    "  ; #{text}\n"
  end

  # Regular instructions: "  OPCODE  op1, op2, ...  ; #ID"
  defp print_instruction(%IrInstruction{opcode: opcode, operands: operands, id: id}) do
    op_name = IrOp.to_string(opcode)

    # Pad the opcode name to 11 characters for visual alignment.
    # E.g. "LOAD_ADDR  " (11 chars), "HALT       " (11 chars)
    padded_name = String.pad_trailing(op_name, 11)

    operand_str =
      operands
      |> Enum.map(&operand_to_string/1)
      |> Enum.join(", ")

    "  #{padded_name}#{operand_str}  ; ##{id}\n"
  end

  # Convert an operand struct to its text representation.
  defp operand_to_string(%IrRegister{} = r), do: IrRegister.to_string(r)
  defp operand_to_string(%IrImmediate{} = i), do: IrImmediate.to_string(i)
  defp operand_to_string(%IrLabel{} = l), do: IrLabel.to_string(l)
end
