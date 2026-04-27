defmodule CodingAdventures.Code128 do
  @moduledoc """
  Dependency-free Code 128 encoder that emits backend-neutral paint scenes.
  """

  alias CodingAdventures.BarcodeLayout1D

  @default_layout_config %{
    module_unit: 4,
    bar_height: 120,
    quiet_zone_modules: 10
  }

  @start_b 104
  @stop 106

  @patterns ~w[
    11011001100 11001101100 11001100110 10010011000 10010001100 10001001100 10011001000 10011000100
    10001100100 11001001000 11001000100 11000100100 10110011100 10011011100 10011001110 10111001100
    10011101100 10011100110 11001110010 11001011100 11001001110 11011100100 11001110100 11101101110
    11101001100 11100101100 11100100110 11101100100 11100110100 11100110010 11011011000 11011000110
    11000110110 10100011000 10001011000 10001000110 10110001000 10001101000 10001100010 11010001000
    11000101000 11000100010 10110111000 10110001110 10001101110 10111011000 10111000110 10001110110
    11101110110 11010001110 11000101110 11011101000 11011100010 11011101110 11101011000 11101000110
    11100010110 11101101000 11101100010 11100011010 11101111010 11001000010 11110001010 10100110000
    10100001100 10010110000 10010000110 10000101100 10000100110 10110010000 10110000100 10011010000
    10011000010 10000110100 10000110010 11000010010 11001010000 11110111010 11000010100 10001111010
    10100111100 10010111100 10010011110 10111100100 10011110100 10011110010 11110100100 11110010100
    11110010010 11011011110 11011110110 11110110110 10101111000 10100011110 10001011110 10111101000
    10111100010 11110101000 11110100010 10111011110 10111101110 11101011110 11110101110 11010000100
    11010010000 11010011100 1100011101011
  ]

  def default_layout_config, do: @default_layout_config
  def default_render_config, do: @default_layout_config

  def normalize_code128_b(data) do
    Enum.each(String.to_charlist(data), fn code ->
      if code < 32 or code > 126 do
        raise ArgumentError, "Code 128 Code Set B supports printable ASCII characters only"
      end
    end)

    data
  end

  def value_for_code128_b_char(char), do: hd(String.to_charlist(char)) - 32

  def compute_code128_checksum(values) do
    weighted_sum =
      values
      |> Enum.with_index()
      |> Enum.map(fn {value, index} -> value * (index + 1) end)
      |> Enum.sum()

    rem(@start_b + weighted_sum, 103)
  end

  def encode_code128_b(data) do
    normalized = normalize_code128_b(data)

    data_symbols =
      normalized
      |> String.graphemes()
      |> Enum.with_index()
      |> Enum.map(fn {char, index} ->
        value = value_for_code128_b_char(char)

        %{
          label: char,
          value: value,
          pattern: Enum.at(@patterns, value),
          source_index: index,
          role: "data"
        }
      end)

    checksum = compute_code128_checksum(Enum.map(data_symbols, & &1.value))

    [
      %{label: "Start B", value: @start_b, pattern: Enum.at(@patterns, @start_b), source_index: -1, role: "start"}
      | data_symbols
    ] ++
      [
        %{label: "Checksum #{checksum}", value: checksum, pattern: Enum.at(@patterns, checksum), source_index: String.length(normalized), role: "check"},
        %{label: "Stop", value: @stop, pattern: Enum.at(@patterns, @stop), source_index: String.length(normalized) + 1, role: "stop"}
      ]
  end

  def expand_code128_runs(data) do
    Enum.flat_map(encode_code128_b(data), fn symbol ->
      BarcodeLayout1D.runs_from_binary_pattern(
        symbol.pattern,
        source_char: symbol.label,
        source_index: symbol.source_index
      )
      |> retag_runs(symbol.role)
    end)
  end

  def layout_code128(data, config \\ @default_layout_config) do
    normalized = normalize_code128_b(data)
    checksum = encode_code128_b(normalized) |> Enum.at(-2) |> Map.fetch!(:value)

    BarcodeLayout1D.layout_barcode_1d(
      expand_code128_runs(normalized),
      config,
      %{
        fill: "#000000",
        background: "#ffffff",
        metadata: %{symbology: "code128", code_set: "B", checksum: checksum}
      }
    )
  end

  def draw_code128(data, config \\ @default_layout_config), do: layout_code128(data, config)

  defp retag_runs(runs, role) do
    Enum.map(runs, fn run -> %{run | role: role, metadata: Map.new(Map.get(run, :metadata, %{}))} end)
  end
end
