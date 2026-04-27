defmodule CodingAdventures.Intel4004Assembler.AssemblerError do
  @moduledoc """
  Exception raised for assembler syntax, resolution, and encoding errors.
  """

  defexception [:message]
end

defmodule CodingAdventures.Intel4004Assembler.ParsedLine do
  @moduledoc """
  Parsed assembly line with optional label, mnemonic, operands, and source text.
  """

  defstruct label: nil, mnemonic: nil, operands: [], source: ""

  @type t :: %__MODULE__{
          label: String.t() | nil,
          mnemonic: String.t() | nil,
          operands: [String.t()],
          source: String.t()
        }
end

defmodule CodingAdventures.Intel4004Assembler.Lexer do
  @moduledoc """
  Line-oriented lexer for Intel 4004 assembly.
  """

  alias CodingAdventures.Intel4004Assembler.ParsedLine

  @label_re ~r/^([A-Za-z_][A-Za-z0-9_]*):/

  @doc "Lex one source line."
  @spec lex_line(String.t()) :: ParsedLine.t()
  def lex_line(source) when is_binary(source) do
    comment_free =
      source
      |> String.split(";", parts: 2)
      |> hd()
      |> String.trim_trailing()

    {label, stripped} = split_label(String.trim_leading(comment_free))

    if stripped == "" do
      %ParsedLine{label: label, source: source}
    else
      {mnemonic, operand_text} = split_mnemonic(stripped)

      %ParsedLine{
        label: label,
        mnemonic: String.upcase(mnemonic),
        operands: parse_operands(operand_text),
        source: source
      }
    end
  end

  @doc "Lex a complete assembly source string."
  @spec lex_program(String.t()) :: [ParsedLine.t()]
  def lex_program(text) when is_binary(text) do
    text
    |> String.replace("\r\n", "\n")
    |> String.split("\n")
    |> Enum.map(&lex_line/1)
  end

  defp split_label(stripped) do
    case Regex.run(@label_re, stripped) do
      [match, label] ->
        {label, stripped |> String.slice(byte_size(match)..-1//1) |> String.trim_leading()}

      _none ->
        {nil, stripped}
    end
  end

  defp split_mnemonic(stripped) do
    case Regex.run(~r/^(\S+)(?:\s+(.*))?$/, stripped) do
      [_all, mnemonic, operand_text] -> {mnemonic, String.trim(operand_text)}
      [_all, mnemonic] -> {mnemonic, ""}
    end
  end

  defp parse_operands(""), do: []

  defp parse_operands(operand_text) do
    operand_text
    |> String.split(",")
    |> Enum.map(&String.trim/1)
    |> Enum.reject(&(&1 == ""))
  end
end

defmodule CodingAdventures.Intel4004Assembler.Encoder do
  @moduledoc """
  Intel 4004 instruction sizing and byte encoding.
  """

  import Bitwise

  alias CodingAdventures.Intel4004Assembler.AssemblerError

  @fixed_opcodes %{
    "NOP" => 0x00,
    "HLT" => 0x01,
    "WRM" => 0xE0,
    "WMP" => 0xE1,
    "WRR" => 0xE2,
    "WR0" => 0xE4,
    "WR1" => 0xE5,
    "WR2" => 0xE6,
    "WR3" => 0xE7,
    "SBM" => 0xE8,
    "RDM" => 0xE9,
    "RDR" => 0xEA,
    "ADM" => 0xEB,
    "RD0" => 0xEC,
    "RD1" => 0xED,
    "RD2" => 0xEE,
    "RD3" => 0xEF,
    "CLB" => 0xF0,
    "CLC" => 0xF1,
    "IAC" => 0xF2,
    "CMC" => 0xF3,
    "CMA" => 0xF4,
    "RAL" => 0xF5,
    "RAR" => 0xF6,
    "TCC" => 0xF7,
    "DAC" => 0xF8,
    "TCS" => 0xF9,
    "STC" => 0xFA,
    "DAA" => 0xFB,
    "KBP" => 0xFC,
    "DCL" => 0xFD
  }

  @one_byte_ops MapSet.new(["INC", "ADD", "SUB", "LD", "XCH", "BBL", "LDM", "SRC", "FIN", "JIN"])
  @two_byte_ops MapSet.new(["JCN", "FIM", "JUN", "JMS", "ISZ", "ADD_IMM"])

  @doc "Return the encoded size of a mnemonic."
  @spec instruction_size(String.t()) :: non_neg_integer()
  def instruction_size(mnemonic) do
    cond do
      Map.has_key?(@fixed_opcodes, mnemonic) -> 1
      MapSet.member?(@one_byte_ops, mnemonic) -> 1
      MapSet.member?(@two_byte_ops, mnemonic) -> 2
      mnemonic == "ORG" -> 0
      true -> raise AssemblerError, "Unknown mnemonic: '#{mnemonic}'"
    end
  end

  @doc "Encode one instruction into a list of bytes."
  @spec encode_instruction(
          String.t(),
          [String.t()],
          %{optional(String.t()) => non_neg_integer()},
          non_neg_integer()
        ) :: [
          byte()
        ]
  def encode_instruction(mnemonic, operands, symbols, pc) do
    cond do
      Map.has_key?(@fixed_opcodes, mnemonic) ->
        expect_operands(mnemonic, operands, 0)
        [Map.fetch!(@fixed_opcodes, mnemonic)]

      mnemonic == "ORG" ->
        []

      mnemonic == "LDM" ->
        immediate = operands |> one_operand(mnemonic) |> resolve_operand(symbols, pc)
        check_range("LDM", immediate, 0, 15)
        [bor(0xD0, immediate)]

      mnemonic == "BBL" ->
        immediate = operands |> one_operand(mnemonic) |> resolve_operand(symbols, pc)
        check_range("BBL", immediate, 0, 15)
        [bor(0xC0, immediate)]

      mnemonic in ["INC", "ADD", "SUB", "LD", "XCH"] ->
        base = %{"INC" => 0x60, "ADD" => 0x80, "SUB" => 0x90, "LD" => 0xA0, "XCH" => 0xB0}
        [bor(Map.fetch!(base, mnemonic), parse_register(one_operand(operands, mnemonic)))]

      mnemonic == "SRC" ->
        [bor(0x20, 2 * parse_pair(one_operand(operands, mnemonic)) + 1)]

      mnemonic == "FIN" ->
        [bor(0x30, 2 * parse_pair(one_operand(operands, mnemonic)))]

      mnemonic == "JIN" ->
        [bor(0x30, 2 * parse_pair(one_operand(operands, mnemonic)) + 1)]

      mnemonic == "FIM" ->
        expect_operands(mnemonic, operands, 2)
        pair = parse_pair(Enum.at(operands, 0))
        immediate = resolve_operand(Enum.at(operands, 1), symbols, pc)
        check_range("FIM", immediate, 0, 255)
        [bor(0x20, 2 * pair), immediate]

      mnemonic == "JCN" ->
        expect_operands(mnemonic, operands, 2)
        condition = resolve_operand(Enum.at(operands, 0), symbols, pc)
        address = resolve_operand(Enum.at(operands, 1), symbols, pc)
        check_range("JCN condition", condition, 0, 15)
        check_range("JCN address", address, 0, 0xFFF)
        [bor(0x10, condition), band(address, 0xFF)]

      mnemonic == "JUN" ->
        address = operands |> one_operand(mnemonic) |> resolve_operand(symbols, pc)
        check_range("JUN", address, 0, 0xFFF)
        [bor(0x40, band(address >>> 8, 0xF)), band(address, 0xFF)]

      mnemonic == "JMS" ->
        address = operands |> one_operand(mnemonic) |> resolve_operand(symbols, pc)
        check_range("JMS", address, 0, 0xFFF)
        [bor(0x50, band(address >>> 8, 0xF)), band(address, 0xFF)]

      mnemonic == "ISZ" ->
        expect_operands(mnemonic, operands, 2)
        register = parse_register(Enum.at(operands, 0))
        address = resolve_operand(Enum.at(operands, 1), symbols, pc)
        check_range("ISZ address", address, 0, 0xFF)
        [bor(0x70, register), band(address, 0xFF)]

      mnemonic == "ADD_IMM" ->
        expect_operands(mnemonic, operands, 3)
        register = parse_register(Enum.at(operands, 1))
        immediate = resolve_operand(Enum.at(operands, 2), symbols, pc)
        check_range("ADD_IMM immediate", immediate, 0, 15)
        [bor(0xD0, immediate), bor(0x80, register)]

      true ->
        raise AssemblerError, "Unknown mnemonic: '#{mnemonic}'"
    end
  end

  defp expect_operands(mnemonic, operands, count) do
    if length(operands) != count do
      raise AssemblerError, "#{mnemonic} expects #{count} operand(s), got #{length(operands)}"
    end
  end

  defp one_operand(operands, mnemonic) do
    expect_operands(mnemonic, operands, 1)
    hd(operands)
  end

  defp parse_register(name) do
    case Regex.run(~r/^R([0-9]|1[0-5])$/i, name) do
      [_match, register] -> String.to_integer(register)
      _none -> raise AssemblerError, "Invalid register name: '#{name}'"
    end
  end

  defp parse_pair(name) do
    case Regex.run(~r/^P([0-7])$/i, name) do
      [_match, pair] -> String.to_integer(pair)
      _none -> raise AssemblerError, "Invalid register pair name: '#{name}'"
    end
  end

  defp resolve_operand("$", _symbols, pc), do: pc

  defp resolve_operand(operand, symbols, _pc) do
    cond do
      numeric?(operand) ->
        parse_number(operand)

      Map.has_key?(symbols, operand) ->
        Map.fetch!(symbols, operand)

      true ->
        raise AssemblerError, "Undefined label: '#{operand}'"
    end
  end

  defp numeric?(operand),
    do: String.match?(operand, ~r/^-?\d+$/) or String.match?(operand, ~r/^0x[0-9a-f]+$/i)

  defp parse_number("0x" <> hex), do: parse_number_with_base(hex, 16, "0x" <> hex)
  defp parse_number("0X" <> hex), do: parse_number_with_base(hex, 16, "0X" <> hex)
  defp parse_number(decimal), do: parse_number_with_base(decimal, 10, decimal)

  defp parse_number_with_base(value, base, original) do
    case Integer.parse(value, base) do
      {parsed, ""} -> parsed
      _error -> raise AssemblerError, "Invalid numeric literal: '#{original}'"
    end
  end

  defp check_range(name, value, low, high) do
    if value < low or value > high do
      raise AssemblerError,
            "#{name} value #{value} (0x#{Integer.to_string(value, 16) |> String.upcase()}) is out of range [#{low}, #{high}]"
    end
  end
end

defmodule CodingAdventures.Intel4004Assembler do
  @moduledoc """
  Two-pass Intel 4004 assembler.
  """

  alias __MODULE__.{AssemblerError, Encoder, Lexer}

  @doc "Assemble source text and return `{:ok, binary}` or `{:error, error}`."
  @spec assemble(String.t()) :: {:ok, binary()} | {:error, AssemblerError.t()}
  def assemble(text) when is_binary(text) do
    {:ok, assemble!(text)}
  rescue
    error in AssemblerError -> {:error, error}
  end

  @doc "Assemble source text and raise `AssemblerError` on failure."
  @spec assemble!(String.t()) :: binary()
  def assemble!(text) when is_binary(text) do
    lines = Lexer.lex_program(text)
    symbols = pass1(lines)
    pass2(lines, symbols)
  end

  @doc "Lex one source line."
  defdelegate lex_line(source), to: Lexer

  @doc "Lex a complete source string."
  defdelegate lex_program(source), to: Lexer

  defp pass1(lines) do
    {symbols, _pc} =
      Enum.reduce(lines, {%{}, 0}, fn line, {symbols, pc} ->
        symbols = if line.label, do: Map.put(symbols, line.label, pc), else: symbols

        cond do
          is_nil(line.mnemonic) ->
            {symbols, pc}

          line.mnemonic == "ORG" ->
            address = line.operands |> org_operand() |> parse_org_address()
            {symbols, address}

          true ->
            {symbols, pc + Encoder.instruction_size(line.mnemonic)}
        end
      end)

    symbols
  end

  defp pass2(lines, symbols) do
    {bytes, _pc} =
      Enum.reduce(lines, {[], 0}, fn line, {bytes, pc} ->
        cond do
          is_nil(line.mnemonic) ->
            {bytes, pc}

          line.mnemonic == "ORG" ->
            address = line.operands |> org_operand() |> parse_org_address()
            padding = if address > pc, do: List.duplicate(0x00, address - pc), else: []
            {bytes ++ padding, address}

          true ->
            encoded = Encoder.encode_instruction(line.mnemonic, line.operands, symbols, pc)
            {bytes ++ encoded, pc + length(encoded)}
        end
      end)

    :erlang.list_to_binary(bytes)
  end

  defp org_operand([]), do: raise(AssemblerError, "ORG requires an address operand")
  defp org_operand([address | _rest]), do: address

  defp parse_org_address(operand) do
    address =
      if String.match?(operand, ~r/^0x[0-9a-f]+$/i) do
        operand |> String.slice(2..-1//1) |> parse_int(16, operand)
      else
        parse_int(operand, 10, operand)
      end

    if address < 0 or address > 0xFFF do
      raise AssemblerError,
            "ORG address 0x#{Integer.to_string(address, 16) |> String.upcase()} exceeds 0xFFF"
    end

    address
  end

  defp parse_int(value, base, original) do
    case Integer.parse(value, base) do
      {parsed, ""} -> parsed
      _error -> raise AssemblerError, "Invalid numeric literal: '#{original}'"
    end
  end
end
