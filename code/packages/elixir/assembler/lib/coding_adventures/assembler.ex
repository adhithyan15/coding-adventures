defmodule CodingAdventures.ArmOpcode do
  @moduledoc """
  ARM data-processing opcodes used by the assembler.
  """

  def and_op, do: 0x0
  def eor, do: 0x1
  def sub, do: 0x2
  def rsb, do: 0x3
  def add, do: 0x4
  def cmp, do: 0xA
  def orr, do: 0xC
  def mov, do: 0xD
end

defmodule CodingAdventures.Operand2 do
  @moduledoc """
  ARM operand2 representation.
  """

  defstruct type: :register, value: 0

  @type t :: %__MODULE__{type: :register | :immediate, value: non_neg_integer()}
end

defmodule CodingAdventures.ArmInstruction do
  @moduledoc """
  Parsed ARM instruction or label.
  """

  defstruct kind: :nop,
            opcode: nil,
            rd: nil,
            rn: nil,
            operand2: nil,
            set_flags: false,
            label: nil

  @type t :: %__MODULE__{
          kind: :data_processing | :load | :store | :nop | :label,
          opcode: integer() | nil,
          rd: integer() | nil,
          rn: integer() | nil,
          operand2: CodingAdventures.Operand2.t() | nil,
          set_flags: boolean(),
          label: String.t() | nil
        }
end

defmodule CodingAdventures.Assembler do
  @moduledoc """
  Small ARM assembler for parsing and encoding a basic instruction subset.
  """

  import Bitwise

  alias CodingAdventures.ArmInstruction
  alias CodingAdventures.ArmOpcode
  alias CodingAdventures.Operand2

  defstruct labels: %{}

  @type t :: %__MODULE__{labels: %{optional(String.t()) => non_neg_integer()}}

  @spec new() :: t()
  def new, do: %__MODULE__{}

  @spec parse_register(String.t()) :: non_neg_integer() | nil
  def parse_register(value) when is_binary(value) do
    value =
      value
      |> String.trim()
      |> String.upcase()

    case value do
      "SP" -> 13
      "LR" -> 14
      "PC" -> 15
      <<"R", rest::binary>> ->
        case Integer.parse(rest) do
          {number, ""} when number in 0..15 -> number
          _ -> nil
        end

      _ ->
        nil
    end
  end

  @spec parse_immediate(String.t()) :: non_neg_integer() | nil
  def parse_immediate(value) when is_binary(value) do
    value =
      value
      |> String.trim()
      |> String.trim_leading("#")
      |> String.trim()

    cond do
      String.starts_with?(value, "0x") or String.starts_with?(value, "0X") ->
        case Integer.parse(String.slice(value, 2..-1//1), 16) do
          {number, ""} -> number
          _ -> nil
        end

      true ->
        case Integer.parse(value) do
          {number, ""} when number >= 0 -> number
          _ -> nil
        end
    end
  end

  @spec parse(t(), String.t()) :: {:ok, t(), [ArmInstruction.t()]} | {:error, String.t()}
  def parse(%__MODULE__{} = assembler, source) when is_binary(source) do
    lines = String.split(source, "\n")

    Enum.reduce_while(lines, {:ok, assembler, [], 0}, fn line, {:ok, acc_asm, instructions, address} ->
      line =
        line
        |> String.split(";", parts: 2)
        |> hd()
        |> String.split("//", parts: 2)
        |> hd()
        |> String.trim()

      cond do
        line == "" ->
          {:cont, {:ok, acc_asm, instructions, address}}

        String.ends_with?(line, ":") ->
          label = String.trim_trailing(line, ":") |> String.trim()
          asm = %{acc_asm | labels: Map.put(acc_asm.labels, label, address)}
          instr = %ArmInstruction{kind: :label, label: label}
          {:cont, {:ok, asm, instructions ++ [instr], address}}

        true ->
          case parse_instruction(line) do
            {:ok, instr} ->
              next_address = if instr.kind == :label, do: address, else: address + 1
              {:cont, {:ok, acc_asm, instructions ++ [instr], next_address}}

            {:error, message} ->
              {:halt, {:error, message}}
          end
      end
    end)
    |> case do
      {:ok, asm, instructions, _address} -> {:ok, asm, instructions}
      {:error, _} = error -> error
    end
  end

  @spec encode(t(), [ArmInstruction.t()]) :: {:ok, [non_neg_integer()]} | {:error, String.t()}
  def encode(%__MODULE__{}, instructions) when is_list(instructions) do
    words =
      Enum.reduce_while(instructions, {:ok, []}, fn instr, {:ok, words} ->
        case encode_instruction(instr) do
          {:skip} -> {:cont, {:ok, words}}
          {:ok, word} -> {:cont, {:ok, words ++ [word]}}
        end
      end)

    case words do
      {:ok, binary} -> {:ok, binary}
      {:error, _} = error -> error
    end
  end

  defp parse_instruction(line) do
    [mnemonic | rest] = String.split(line, ~r/\s+/, parts: 2)
    mnemonic = String.upcase(mnemonic)
    operands = if rest == [], do: [], else: split_operands(hd(rest))

    case mnemonic do
      "NOP" ->
        {:ok, %ArmInstruction{kind: :nop}}

      "MOV" ->
        with {:ok, [rd, operand]} <- expect_operands(mnemonic, operands, 2),
             {:ok, rd} <- parse_required_register(rd),
             {:ok, operand2} <- parse_operand2(operand) do
          {:ok,
           %ArmInstruction{
             kind: :data_processing,
             opcode: ArmOpcode.mov(),
             rd: rd,
             rn: nil,
             operand2: operand2,
             set_flags: false
           }}
        end

      mnemonic when mnemonic in ["ADD", "SUB", "AND", "ORR", "EOR", "RSB"] ->
        with {:ok, [rd, rn, operand]} <- expect_operands(mnemonic, operands, 3),
             {:ok, rd} <- parse_required_register(rd),
             {:ok, rn} <- parse_required_register(rn),
             {:ok, operand2} <- parse_operand2(operand) do
          {:ok,
           %ArmInstruction{
             kind: :data_processing,
             opcode: mnemonic_to_opcode(mnemonic),
             rd: rd,
             rn: rn,
             operand2: operand2,
             set_flags: false
           }}
        end

      "CMP" ->
        with {:ok, [rn, operand]} <- expect_operands(mnemonic, operands, 2),
             {:ok, rn} <- parse_required_register(rn),
             {:ok, operand2} <- parse_operand2(operand) do
          {:ok,
           %ArmInstruction{
             kind: :data_processing,
             opcode: ArmOpcode.cmp(),
             rd: nil,
             rn: rn,
             operand2: operand2,
             set_flags: true
           }}
        end

      "LDR" ->
        with {:ok, [rd, base]} <- expect_operands(mnemonic, operands, 2),
             {:ok, rd} <- parse_required_register(rd),
             {:ok, rn} <- parse_bracket_register(base) do
          {:ok, %ArmInstruction{kind: :load, rd: rd, rn: rn}}
        end

      "STR" ->
        with {:ok, [rd, base]} <- expect_operands(mnemonic, operands, 2),
             {:ok, rd} <- parse_required_register(rd),
             {:ok, rn} <- parse_bracket_register(base) do
          {:ok, %ArmInstruction{kind: :store, rd: rd, rn: rn}}
        end

      _ ->
        {:error, "Unknown mnemonic: #{mnemonic}"}
    end
  end

  defp encode_instruction(%ArmInstruction{kind: :label}), do: {:skip}
  defp encode_instruction(%ArmInstruction{kind: :nop}), do: {:ok, 0xE1A00000}

  defp encode_instruction(%ArmInstruction{kind: :load, rd: rd, rn: rn}) do
    {:ok, 0xE5900000 ||| (rn <<< 16) ||| (rd <<< 12)}
  end

  defp encode_instruction(%ArmInstruction{kind: :store, rd: rd, rn: rn}) do
    {:ok, 0xE5800000 ||| (rn <<< 16) ||| (rd <<< 12)}
  end

  defp encode_instruction(%ArmInstruction{kind: :data_processing} = instr) do
    cond = 0xE <<< 28
    rd = (instr.rd || 0) <<< 12
    rn = (instr.rn || 0) <<< 16
    s_bit = if(instr.set_flags, do: 1 <<< 20, else: 0)
    opcode = instr.opcode <<< 21

    {i_bit, operand2} =
      case instr.operand2 do
        %Operand2{type: :immediate, value: value} -> {1 <<< 25, Bitwise.band(value, 0xFFF)}
        %Operand2{type: :register, value: value} -> {0, Bitwise.band(value, 0xF)}
      end

    {:ok, cond ||| i_bit ||| opcode ||| s_bit ||| rn ||| rd ||| operand2}
  end

  defp split_operands(""), do: []
  defp split_operands(value), do: value |> String.split(",") |> Enum.map(&String.trim/1)

  defp expect_operands(mnemonic, operands, count) do
    if length(operands) == count do
      {:ok, operands}
    else
      {:error, "#{mnemonic}: expected #{count} operands, got #{length(operands)}"}
    end
  end

  defp parse_required_register(value) do
    case parse_register(value) do
      nil -> {:error, "Invalid register: #{value}"}
      register -> {:ok, register}
    end
  end

  defp parse_bracket_register(value) do
    value = value |> String.trim() |> String.trim_leading("[") |> String.trim_trailing("]") |> String.trim()
    parse_required_register(value)
  end

  defp parse_operand2(value) do
    cond do
      String.starts_with?(String.trim(value), "#") ->
        case parse_immediate(value) do
          nil -> {:error, "Invalid immediate: #{value}"}
          immediate -> {:ok, %Operand2{type: :immediate, value: immediate}}
        end

      parse_register(value) != nil ->
        {:ok, %Operand2{type: :register, value: parse_register(value)}}

      true ->
        {:error, "Parse error: Cannot parse operand: #{value}"}
    end
  end

  defp mnemonic_to_opcode("ADD"), do: ArmOpcode.add()
  defp mnemonic_to_opcode("SUB"), do: ArmOpcode.sub()
  defp mnemonic_to_opcode("AND"), do: ArmOpcode.and_op()
  defp mnemonic_to_opcode("ORR"), do: ArmOpcode.orr()
  defp mnemonic_to_opcode("EOR"), do: ArmOpcode.eor()
  defp mnemonic_to_opcode("RSB"), do: ArmOpcode.rsb()
end
