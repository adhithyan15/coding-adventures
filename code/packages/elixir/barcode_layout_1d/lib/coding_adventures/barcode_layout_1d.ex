defmodule CodingAdventures.BarcodeLayout1D do
  @moduledoc """
  Pure 1D barcode layout utilities.
  """

  alias CodingAdventures.PaintInstructions

  @default_layout_config %{
    module_unit: 4,
    bar_height: 120,
    quiet_zone_modules: 10
  }

  @default_paint_options %{
    fill: "#000000",
    background: "#ffffff",
    metadata: %{}
  }

  def default_layout_config, do: @default_layout_config
  def default_paint_options, do: @default_paint_options

  def runs_from_binary_pattern(pattern, opts \\ []) do
    bar_char = Keyword.get(opts, :bar_char, "1")
    space_char = Keyword.get(opts, :space_char, "0")
    source_char = Keyword.get(opts, :source_char, "")
    source_index = Keyword.get(opts, :source_index, 0)
    metadata = Keyword.get(opts, :metadata, %{})

    case String.graphemes(pattern) do
      [] ->
        []

      [first | rest] ->
        {runs, current, count} =
          Enum.reduce(rest, {[], first, 1}, fn token, {runs, current, count} ->
            if token == current do
              {runs, current, count + 1}
            else
              {[build_binary_run(current, count, bar_char, space_char, source_char, source_index, metadata) | runs], token, 1}
            end
          end)

        Enum.reverse([build_binary_run(current, count, bar_char, space_char, source_char, source_index, metadata) | runs])
    end
  end

  def runs_from_width_pattern(pattern, colors, opts) do
    source_char = Keyword.fetch!(opts, :source_char)
    source_index = Keyword.fetch!(opts, :source_index)
    narrow_modules = Keyword.get(opts, :narrow_modules, 1)
    wide_modules = Keyword.get(opts, :wide_modules, 3)
    role = Keyword.get(opts, :role, "data")
    metadata = Keyword.get(opts, :metadata, %{})

    if String.length(pattern) != length(colors) do
      raise ArgumentError, "pattern length must match colors length"
    end

    if narrow_modules <= 0 or wide_modules <= 0 do
      raise ArgumentError, "narrow_modules and wide_modules must be positive integers"
    end

    pattern
    |> String.graphemes()
    |> Enum.with_index()
    |> Enum.map(fn {token, index} ->
      modules =
        case token do
          "N" -> narrow_modules
          "W" -> wide_modules
          _ -> raise ArgumentError, "width pattern contains unsupported token: #{inspect(token)}"
        end

      %{
        color: Enum.at(colors, index),
        modules: modules,
        source_char: source_char,
        source_index: source_index,
        role: role,
        metadata: metadata
      }
    end)
  end

  def layout_barcode_1d(runs, config \\ @default_layout_config, options \\ @default_paint_options) do
    validate_layout_config!(config)

    quiet_zone_width = config.module_unit * config.quiet_zone_modules

    {instructions, cursor_x} =
      Enum.reduce(runs, {[], quiet_zone_width}, fn run, {instructions, cursor_x} ->
        validate_run!(run)
        width = run.modules * config.module_unit

        instructions =
          if run.color == "bar" do
            instructions ++
              [
                PaintInstructions.paint_rect(
                  cursor_x,
                  0,
                  width,
                  config.bar_height,
                  options.fill,
                  %{
                    source_char: run.source_char,
                    source_index: run.source_index,
                    modules: run.modules,
                    role: run.role
                  }
                  |> Map.merge(Map.get(run, :metadata, %{}))
                )
              ]
          else
            instructions
          end

        {instructions, cursor_x + width}
      end)

    PaintInstructions.paint_scene(
      cursor_x + quiet_zone_width,
      config.bar_height,
      instructions,
      options.background,
      %{
        content_width: cursor_x - quiet_zone_width,
        quiet_zone_width: quiet_zone_width,
        module_unit: config.module_unit,
        bar_height: config.bar_height
      }
      |> Map.merge(Map.get(options, :metadata, %{}))
    )
  end

  def draw_one_dimensional_barcode(runs, config \\ @default_layout_config, options \\ @default_paint_options) do
    layout_barcode_1d(runs, config, options)
  end

  defp build_binary_run(token, modules, bar_char, space_char, source_char, source_index, metadata) do
    color =
      cond do
        token == bar_char -> "bar"
        token == space_char -> "space"
        true -> raise ArgumentError, "binary pattern contains unsupported token: #{inspect(token)}"
      end

    %{
      color: color,
      modules: modules,
      source_char: source_char,
      source_index: source_index,
      role: "data",
      metadata: metadata
    }
  end

  defp validate_layout_config!(config) do
    if config.module_unit <= 0, do: raise(ArgumentError, "module_unit must be a positive integer")
    if config.bar_height <= 0, do: raise(ArgumentError, "bar_height must be a positive integer")
    if config.quiet_zone_modules < 0, do: raise(ArgumentError, "quiet_zone_modules must be zero or a positive integer")
  end

  defp validate_run!(run) do
    if run.color not in ["bar", "space"], do: raise(ArgumentError, "run color must be 'bar' or 'space'")
    if run.modules <= 0, do: raise(ArgumentError, "run modules must be a positive integer")
  end
end
