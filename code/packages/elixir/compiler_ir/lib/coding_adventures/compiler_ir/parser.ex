defmodule CodingAdventures.CompilerIr.Parser do
  @moduledoc """
  IR Parser — text → `IrProgram`.

  The parser reads the canonical IR text format (produced by `Printer.print/1`)
  and reconstructs an `IrProgram`. This enables:

  1. **Golden-file testing** — load an expected `.ir` file, parse it, compare.
  2. **Roundtrip verification** — `parse(print(program)) == program`.
  3. **Manual IR authoring** — write IR by hand for testing backends.

  ## Parsing strategy

  The parser processes the text line-by-line:

  1. Lines starting with `.version` set the program version.
  2. Lines starting with `.data` add a data declaration.
  3. Lines starting with `.entry` set the entry label.
  4. Lines ending with `:` define a label (LABEL instruction).
  5. Lines starting with `;` are standalone comments (COMMENT instruction).
  6. Lines starting with whitespace are regular instructions.
  7. Blank lines are skipped.

  Each instruction line is split into: opcode, operands, and optional
  `; #N` ID comment. Operands are parsed as:
  - `vN` (starts with `v` + digits) → `IrRegister{index: N}`
  - Integer literal → `IrImmediate{value: N}`
  - Anything else → `IrLabel{name: str}`

  ## Limits

  To prevent denial-of-service from adversarial input:
  - Maximum 1,000,000 lines.
  - Maximum 16 operands per instruction.
  - Maximum register index 65,535.
  """

  alias CodingAdventures.CompilerIr.{IrProgram, IrInstruction, IrDataDecl, IrOp}
  alias CodingAdventures.CompilerIr.{IrRegister, IrImmediate, IrLabel}

  @max_lines 1_000_000
  @max_operands_per_instr 16
  @max_register_index 65_535

  @doc """
  Parse IR text into an `IrProgram`.

  Returns `{:ok, program}` on success, `{:error, message}` on failure.

  ## Examples

      iex> text = ".version 1\\n\\n.entry _start\\n\\n_start:\\n  HALT  ; #0\\n"
      iex> {:ok, p} = Parser.parse(text)
      iex> p.entry_label
      "_start"
  """
  @spec parse(String.t()) :: {:ok, IrProgram.t()} | {:error, String.t()}
  def parse(text) when is_binary(text) do
    lines = String.split(text, "\n")

    if length(lines) > @max_lines do
      {:error, "input too large: #{length(lines)} lines (max #{@max_lines})"}
    else
      parse_lines(lines, %IrProgram{version: 1}, 1)
    end
  end

  # ── Line-by-line processing ──────────────────────────────────────────────────

  defp parse_lines([], program, _line_num), do: {:ok, program}

  defp parse_lines([line | rest], program, line_num) do
    trimmed = String.trim(line)

    cond do
      # Blank lines — skip
      trimmed == "" ->
        parse_lines(rest, program, line_num + 1)

      # .version directive
      String.starts_with?(trimmed, ".version") ->
        case parse_version(trimmed, line_num) do
          {:ok, version} ->
            parse_lines(rest, %{program | version: version}, line_num + 1)

          {:error, _} = err ->
            err
        end

      # .data directive
      String.starts_with?(trimmed, ".data") ->
        case parse_data(trimmed, line_num) do
          {:ok, decl} ->
            parse_lines(rest, IrProgram.add_data(program, decl), line_num + 1)

          {:error, _} = err ->
            err
        end

      # .entry directive
      String.starts_with?(trimmed, ".entry") ->
        case parse_entry(trimmed, line_num) do
          {:ok, entry_label} ->
            parse_lines(rest, %{program | entry_label: entry_label}, line_num + 1)

          {:error, _} = err ->
            err
        end

      # Label definition — ends with ":" and doesn't start with ";"
      String.ends_with?(trimmed, ":") and not String.starts_with?(trimmed, ";") ->
        label_name = String.trim_trailing(trimmed, ":")

        instr = %IrInstruction{
          opcode: :label,
          operands: [%IrLabel{name: label_name}],
          id: -1
        }

        parse_lines(rest, IrProgram.add_instruction(program, instr), line_num + 1)

      # Standalone comment line — starts with ";"
      String.starts_with?(trimmed, ";") ->
        comment_text = String.trim_leading(trimmed, ";") |> String.trim()

        # Only add as COMMENT instruction if it's not an ID comment like "; #3"
        if not String.starts_with?(comment_text, "#") do
          instr = %IrInstruction{
            opcode: :comment,
            operands: [%IrLabel{name: comment_text}],
            id: -1
          }

          parse_lines(rest, IrProgram.add_instruction(program, instr), line_num + 1)
        else
          parse_lines(rest, program, line_num + 1)
        end

      # Instruction line (starts with whitespace, or anything else)
      true ->
        case parse_instruction_line(trimmed, line_num) do
          {:ok, instr} ->
            parse_lines(rest, IrProgram.add_instruction(program, instr), line_num + 1)

          {:error, _} = err ->
            err
        end
    end
  end

  # ── Directive parsers ────────────────────────────────────────────────────────

  defp parse_version(line, line_num) do
    parts = String.split(line)

    case parts do
      [".version", ver_str] ->
        case Integer.parse(ver_str) do
          {v, ""} -> {:ok, v}
          _ -> {:error, "line #{line_num}: invalid version number: #{inspect(ver_str)}"}
        end

      _ ->
        {:error, "line #{line_num}: invalid .version directive: #{inspect(line)}"}
    end
  end

  defp parse_data(line, line_num) do
    parts = String.split(line)

    case parts do
      [".data", label, size_str, init_str] ->
        with {size, ""} <- Integer.parse(size_str),
             {init, ""} <- Integer.parse(init_str) do
          {:ok, %IrDataDecl{label: label, size: size, init: init}}
        else
          _ ->
            {:error,
             "line #{line_num}: invalid .data directive (bad size or init): #{inspect(line)}"}
        end

      _ ->
        {:error, "line #{line_num}: invalid .data directive: #{inspect(line)}"}
    end
  end

  defp parse_entry(line, line_num) do
    parts = String.split(line)

    case parts do
      [".entry", label] -> {:ok, label}
      _ -> {:error, "line #{line_num}: invalid .entry directive: #{inspect(line)}"}
    end
  end

  # ── Instruction line parser ──────────────────────────────────────────────────

  defp parse_instruction_line(line, line_num) do
    # Split off the "; #N" ID comment if present.
    # We use the last occurrence of "; #" to handle comments in label names.
    {instruction_part, id} =
      case :binary.matches(line, "; #") do
        [] ->
          {String.trim(line), -1}

        matches ->
          # Take the last match
          {pos, _len} = List.last(matches)
          id_str = String.trim(binary_part(line, pos + 3, byte_size(line) - pos - 3))
          instr_part = String.trim(binary_part(line, 0, pos))

          id_val =
            case Integer.parse(id_str) do
              {n, ""} -> n
              _ -> -1
            end

          {instr_part, id_val}
      end

    if instruction_part == "" do
      {:error, "line #{line_num}: empty instruction"}
    else
      parse_opcode_and_operands(instruction_part, id, line_num)
    end
  end

  defp parse_opcode_and_operands(instruction_part, id, line_num) do
    # Split into opcode name and the rest
    case String.split(instruction_part, ~r/\s+/, parts: 2) do
      [opcode_name] ->
        # No operands
        case IrOp.parse(opcode_name) do
          {:ok, opcode} ->
            {:ok, %IrInstruction{opcode: opcode, operands: [], id: id}}

          {:error, :unknown_opcode} ->
            {:error, "line #{line_num}: unknown opcode #{inspect(opcode_name)}"}
        end

      [opcode_name, operand_str] ->
        case IrOp.parse(opcode_name) do
          {:ok, opcode} ->
            case parse_operands(operand_str, line_num) do
              {:ok, operands} ->
                {:ok, %IrInstruction{opcode: opcode, operands: operands, id: id}}

              {:error, _} = err ->
                err
            end

          {:error, :unknown_opcode} ->
            {:error, "line #{line_num}: unknown opcode #{inspect(opcode_name)}"}
        end
    end
  end

  defp parse_operands(operand_str, line_num) do
    parts =
      operand_str
      |> String.split(",")
      |> Enum.map(&String.trim/1)
      |> Enum.reject(&(&1 == ""))

    if length(parts) > @max_operands_per_instr do
      {:error,
       "line #{line_num}: too many operands (#{length(parts)}, max #{@max_operands_per_instr})"}
    else
      parts
      |> Enum.reduce_while({:ok, []}, fn part, {:ok, acc} ->
        case parse_operand(part, line_num) do
          {:ok, op} -> {:cont, {:ok, acc ++ [op]}}
          {:error, _} = err -> {:halt, err}
        end
      end)
    end
  end

  # Parse a single operand string.
  #
  # Rules (in priority order):
  #   1. "vN" where N is digits → IrRegister{index: N}
  #   2. Parseable integer → IrImmediate{value: N}
  #   3. Anything else → IrLabel{name: str}
  defp parse_operand(s, line_num) do
    cond do
      # Register: v0, v1, v2, ...
      byte_size(s) > 1 and binary_part(s, 0, 1) == "v" ->
        rest = binary_part(s, 1, byte_size(s) - 1)

        case Integer.parse(rest) do
          {idx, ""} when idx >= 0 and idx <= @max_register_index ->
            {:ok, %IrRegister{index: idx}}

          {idx, ""} ->
            {:error,
             "line #{line_num}: register index #{idx} out of range (max #{@max_register_index})"}

          _ ->
            # Not a valid register number — fall through to label
            {:ok, %IrLabel{name: s}}
        end

      # Immediate: 42, -1, 255
      true ->
        case Integer.parse(s) do
          {val, ""} ->
            {:ok, %IrImmediate{value: val}}

          _ ->
            # Label: _start, loop_0_end, tape
            {:ok, %IrLabel{name: s}}
        end
    end
  end
end
