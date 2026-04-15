defmodule CodingAdventures.Code39 do
  @moduledoc """
  Dependency-free Code 39 encoder that emits backend-neutral paint scenes.
  """

  alias CodingAdventures.BarcodeLayout1D

  @default_layout_config %{
    module_unit: 4,
    bar_height: 120,
    quiet_zone_modules: 10
  }

  @patterns %{
    "0" => "bwbWBwBwb", "1" => "BwbWbwbwB", "2" => "bwBWbwbwB", "3" => "BwBWbwbwb",
    "4" => "bwbWBwbwB", "5" => "BwbWBwbwb", "6" => "bwBWBwbwb", "7" => "bwbWbwBwB",
    "8" => "BwbWbwBwb", "9" => "bwBWbwBwb", "A" => "BwbwbWbwB", "B" => "bwBwbWbwB",
    "C" => "BwBwbWbwb", "D" => "bwbwBWbwB", "E" => "BwbwBWbwb", "F" => "bwBwBWbwb",
    "G" => "bwbwbWBwB", "H" => "BwbwbWBwb", "I" => "bwBwbWBwb", "J" => "bwbwBWBwb",
    "K" => "BwbwbwbWB", "L" => "bwBwbwbWB", "M" => "BwBwbwbWb", "N" => "bwbwBwbWB",
    "O" => "BwbwBwbWb", "P" => "bwBwBwbWb", "Q" => "bwbwbwBWB", "R" => "BwbwbwBWb",
    "S" => "bwBwbwBWb", "T" => "bwbwBwBWb", "U" => "BWbwbwbwB", "V" => "bWBwbwbwB",
    "W" => "BWBwbwbwb", "X" => "bWbwBwbwB", "Y" => "BWbwBwbwb", "Z" => "bWBwBwbwb",
    "-" => "bWbwbwBwB", "." => "BWbwbwBwb", " " => "bWBwbwBwb", "$" => "bWbWbWbwb",
    "/" => "bWbWbwbWb", "+" => "bWbwbWbWb", "%" => "bwbWbWbWb", "*" => "bWbwBwBwb"
  }

  @bar_space_colors ["bar", "space", "bar", "space", "bar", "space", "bar", "space", "bar"]

  def default_layout_config, do: @default_layout_config
  def default_render_config, do: @default_layout_config

  def normalize_code39(data) do
    normalized = String.upcase(data)

    Enum.each(String.graphemes(normalized), fn char ->
      cond do
        char == "*" ->
          raise ArgumentError, ~s(input must not contain "*" because it is reserved for start/stop)

        not Map.has_key?(@patterns, char) ->
          raise ArgumentError, ~s(invalid character: "#{char}" is not supported by Code 39)

        true ->
          :ok
      end
    end)

    normalized
  end

  def encode_code39_char(char) do
    pattern = Map.fetch!(@patterns, char)

    %{
      char: char,
      is_start_stop: char == "*",
      pattern:
        pattern
        |> String.graphemes()
        |> Enum.map_join(fn part -> if String.upcase(part) == part, do: "W", else: "N" end)
    }
  end

  def encode_code39(data) do
    normalized = normalize_code39(data)
    ("*" <> normalized <> "*") |> String.graphemes() |> Enum.map(&encode_code39_char/1)
  end

  def expand_code39_runs(data) do
    encode_code39(data)
    |> Enum.with_index()
    |> Enum.flat_map(fn {encoded_char, source_index} ->
      runs = BarcodeLayout1D.runs_from_width_pattern(
        encoded_char.pattern,
        @bar_space_colors,
        source_char: encoded_char.char,
        source_index: source_index
      )

      if source_index < length(encode_code39(data)) - 1 do
        runs ++
          [
            %{
              color: "space",
              modules: 1,
              source_char: encoded_char.char,
              source_index: source_index,
              role: "inter-character-gap",
              metadata: %{}
            }
          ]
      else
        runs
      end
    end)
  end

  def layout_code39(data, config \\ @default_layout_config) do
    normalized = normalize_code39(data)
    BarcodeLayout1D.layout_barcode_1d(
      expand_code39_runs(normalized),
      config,
      %{
        fill: "#000000",
        background: "#ffffff",
        metadata: %{symbology: "code39", data: normalized}
      }
    )
  end

  def draw_code39(data, config \\ @default_layout_config), do: layout_code39(data, config)
end
