defmodule CodingAdventures.Code39 do
  @moduledoc """
  Dependency-free Code 39 encoder that emits backend-neutral draw scenes.
  """

  alias CodingAdventures.DrawInstructions

  @default_render_config %{
    narrow_unit: 4,
    wide_unit: 12,
    bar_height: 120,
    quiet_zone_units: 10,
    include_human_readable_text: true
  }

  @text_margin 8
  @text_font_size 16
  @text_block_height @text_margin + @text_font_size + 4

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

  def default_render_config, do: @default_render_config

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
    colors = ["bar", "space", "bar", "space", "bar", "space", "bar", "space", "bar"]

    encode_code39(data)
    |> Enum.with_index()
    |> Enum.flat_map(fn {encoded_char, source_index} ->
      runs =
        encoded_char.pattern
        |> String.graphemes()
        |> Enum.with_index()
        |> Enum.map(fn {element, element_index} ->
          %{
            color: Enum.at(colors, element_index),
            width: if(element == "W", do: "wide", else: "narrow"),
            source_char: encoded_char.char,
            source_index: source_index,
            is_inter_character_gap: false
          }
        end)

      if source_index < length(encode_code39(data)) - 1 do
        runs ++
          [
            %{
              color: "space",
              width: "narrow",
              source_char: encoded_char.char,
              source_index: source_index,
              is_inter_character_gap: true
            }
          ]
      else
        runs
      end
    end)
  end

  def draw_code39(data, config \\ @default_render_config) do
    normalized = normalize_code39(data)
    quiet_zone_width = config.quiet_zone_units * config.narrow_unit

    {instructions, cursor_x} =
      Enum.reduce(expand_code39_runs(normalized), {[], quiet_zone_width}, fn run, {instructions, cursor_x} ->
        width = if run.width == "wide", do: config.wide_unit, else: config.narrow_unit

        instructions =
          if run.color == "bar" do
            instructions ++
              [
                DrawInstructions.draw_rect(cursor_x, 0, width, config.bar_height, "#000000", %{
                  char: run.source_char,
                  index: run.source_index
                })
              ]
          else
            instructions
          end

        {instructions, cursor_x + width}
      end)

    instructions =
      if config.include_human_readable_text do
        instructions ++
          [
            DrawInstructions.draw_text(
              div(cursor_x + quiet_zone_width, 2),
              config.bar_height + @text_margin + @text_font_size - 2,
              normalized,
              %{role: "label"}
            )
          ]
      else
        instructions
      end

    DrawInstructions.create_scene(
      cursor_x + quiet_zone_width,
      config.bar_height + if(config.include_human_readable_text, do: @text_block_height, else: 0),
      instructions,
      "#ffffff",
      %{label: "Code 39 barcode for #{normalized}", symbology: "code39"}
    )
  end
end
