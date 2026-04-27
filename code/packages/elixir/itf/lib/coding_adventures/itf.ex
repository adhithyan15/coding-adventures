defmodule CodingAdventures.Itf do
  @moduledoc """
  Dependency-free ITF encoder that emits backend-neutral paint scenes.
  """

  alias CodingAdventures.BarcodeLayout1D

  @default_layout_config %{
    module_unit: 4,
    bar_height: 120,
    quiet_zone_modules: 10
  }

  @start_pattern "1010"
  @stop_pattern "11101"
  @digit_patterns ~w[00110 10001 01001 11000 00101 10100 01100 00011 10010 01010]

  def default_layout_config, do: @default_layout_config
  def default_render_config, do: @default_layout_config

  def normalize_itf(data) do
    unless String.match?(data, ~r/^\d+$/) do
      raise ArgumentError, "ITF input must contain digits only"
    end

    if data == "" or rem(String.length(data), 2) != 0 do
      raise ArgumentError, "ITF input must contain an even number of digits"
    end

    data
  end

  def encode_itf(data) do
    normalize_itf(data)
    |> String.graphemes()
    |> Enum.chunk_every(2)
    |> Enum.with_index()
    |> Enum.map(fn {pair_chars, index} ->
      pair = Enum.join(pair_chars)
      bar_pattern = Enum.at(@digit_patterns, String.at(pair, 0) |> String.to_integer())
      space_pattern = Enum.at(@digit_patterns, String.at(pair, 1) |> String.to_integer())

      binary_pattern =
        bar_pattern
        |> String.graphemes()
        |> Enum.zip(String.graphemes(space_pattern))
        |> Enum.map_join(fn {bar_marker, space_marker} ->
          "#{if(bar_marker == "1", do: "111", else: "1")}#{if(space_marker == "1", do: "000", else: "0")}"
        end)

      %{
        pair: pair,
        bar_pattern: bar_pattern,
        space_pattern: space_pattern,
        binary_pattern: binary_pattern,
        source_index: index
      }
    end)
  end

  def expand_itf_runs(data) do
    encoded_pairs = encode_itf(data)

    retag_runs(BarcodeLayout1D.runs_from_binary_pattern(@start_pattern, source_char: "start", source_index: -1), "start") ++
      Enum.flat_map(encoded_pairs, fn entry ->
        BarcodeLayout1D.runs_from_binary_pattern(
          entry.binary_pattern,
          source_char: entry.pair,
          source_index: entry.source_index
        )
        |> retag_runs("data")
      end) ++
      retag_runs(BarcodeLayout1D.runs_from_binary_pattern(@stop_pattern, source_char: "stop", source_index: -2), "stop")
  end

  def layout_itf(data, config \\ @default_layout_config) do
    normalized = normalize_itf(data)

    BarcodeLayout1D.layout_barcode_1d(
      expand_itf_runs(normalized),
      config,
      %{
        fill: "#000000",
        background: "#ffffff",
        metadata: %{symbology: "itf", pair_count: div(String.length(normalized), 2)}
      }
    )
  end

  def draw_itf(data, config \\ @default_layout_config), do: layout_itf(data, config)

  defp retag_runs(runs, role) do
    Enum.map(runs, fn run -> %{run | role: role, metadata: Map.new(Map.get(run, :metadata, %{}))} end)
  end
end
