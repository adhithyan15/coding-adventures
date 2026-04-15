defmodule CodingAdventures.UpcA do
  @moduledoc """
  Dependency-free UPC-A encoder that emits backend-neutral paint scenes.
  """

  alias CodingAdventures.BarcodeLayout1D

  @default_layout_config %{
    module_unit: 4,
    bar_height: 120,
    quiet_zone_modules: 10
  }

  @side_guard "101"
  @center_guard "01010"

  @digit_patterns %{
    "L" => ~w[0001101 0011001 0010011 0111101 0100011 0110001 0101111 0111011 0110111 0001011],
    "R" => ~w[1110010 1100110 1101100 1000010 1011100 1001110 1010000 1000100 1001000 1110100]
  }

  def default_layout_config, do: @default_layout_config
  def default_render_config, do: @default_layout_config

  def compute_upc_a_check_digit(payload11) do
    assert_digits!(payload11, [11], "UPC-A input must contain 11 digits or 12 digits")

    {odd_sum, even_sum} =
      payload11
      |> String.graphemes()
      |> Enum.with_index()
      |> Enum.reduce({0, 0}, fn {digit, index}, {odd_sum, even_sum} ->
        if rem(index, 2) == 0 do
          {odd_sum + String.to_integer(digit), even_sum}
        else
          {odd_sum, even_sum + String.to_integer(digit)}
        end
      end)

    Integer.to_string(rem(10 - rem(odd_sum * 3 + even_sum, 10), 10))
  end

  def normalize_upc_a(data) do
    assert_digits!(data, [11, 12], "UPC-A input must contain 11 digits or 12 digits")

    if String.length(data) == 11 do
      data <> compute_upc_a_check_digit(data)
    else
      expected = compute_upc_a_check_digit(String.slice(data, 0, 11))
      actual = String.at(data, 11)

      if expected == actual do
        data
      else
        raise ArgumentError, "Invalid UPC-A check digit: expected #{expected} but received #{actual}"
      end
    end
  end

  def encode_upc_a(data) do
    normalize_upc_a(data)
    |> String.graphemes()
    |> Enum.with_index()
    |> Enum.map(fn {digit, index} ->
      encoding = if(index < 6, do: "L", else: "R")

      %{
        digit: digit,
        encoding: encoding,
        pattern: @digit_patterns |> Map.fetch!(encoding) |> Enum.at(String.to_integer(digit)),
        source_index: index,
        role: if(index == 11, do: "check", else: "data")
      }
    end)
  end

  def expand_upc_a_runs(data) do
    encoded_digits = encode_upc_a(data)

    retag_runs(BarcodeLayout1D.runs_from_binary_pattern(@side_guard, source_char: "start", source_index: -1), "guard") ++
      expand_upc_a_side(Enum.take(encoded_digits, 6)) ++
      retag_runs(BarcodeLayout1D.runs_from_binary_pattern(@center_guard, source_char: "center", source_index: -2), "guard") ++
      expand_upc_a_side(Enum.drop(encoded_digits, 6)) ++
      retag_runs(BarcodeLayout1D.runs_from_binary_pattern(@side_guard, source_char: "end", source_index: -3), "guard")
  end

  def layout_upc_a(data, config \\ @default_layout_config) do
    normalized = normalize_upc_a(data)

    BarcodeLayout1D.layout_barcode_1d(
      expand_upc_a_runs(normalized),
      config,
      %{
        fill: "#000000",
        background: "#ffffff",
        metadata: %{symbology: "upc-a", content_modules: 95}
      }
    )
  end

  def draw_upc_a(data, config \\ @default_layout_config), do: layout_upc_a(data, config)

  defp expand_upc_a_side(entries) do
    Enum.flat_map(entries, fn entry ->
      BarcodeLayout1D.runs_from_binary_pattern(
        entry.pattern,
        source_char: entry.digit,
        source_index: entry.source_index
      )
      |> retag_runs(entry.role)
    end)
  end

  defp assert_digits!(data, lengths, length_message) do
    unless String.match?(data, ~r/^\d+$/) do
      raise ArgumentError, "UPC-A input must contain digits only"
    end

    unless String.length(data) in lengths do
      raise ArgumentError, length_message
    end
  end

  defp retag_runs(runs, role) do
    Enum.map(runs, fn run -> %{run | role: role, metadata: Map.new(Map.get(run, :metadata, %{}))} end)
  end
end
