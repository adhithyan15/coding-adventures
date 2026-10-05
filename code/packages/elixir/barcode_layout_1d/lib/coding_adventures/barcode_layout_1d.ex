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
              {[
                 build_binary_run(
                   current,
                   count,
                   bar_char,
                   space_char,
                   source_char,
                   source_index,
                   metadata
                 )
                 | runs
               ], token, 1}
            end
          end)

        Enum.reverse([
          build_binary_run(
            current,
            count,
            bar_char,
            space_char,
            source_char,
            source_index,
            metadata
          )
          | runs
        ])
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

  def draw_one_dimensional_barcode(
        runs,
        config \\ @default_layout_config,
        options \\ @default_paint_options
      ) do
    layout_barcode_1d(runs, config, options)
  end

  defp build_binary_run(token, modules, bar_char, space_char, source_char, source_index, metadata) do
    color =
      cond do
        token == bar_char ->
          "bar"

        token == space_char ->
          "space"

        true ->
          raise ArgumentError, "binary pattern contains unsupported token: #{inspect(token)}"
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

    if config.quiet_zone_modules < 0,
      do: raise(ArgumentError, "quiet_zone_modules must be zero or a positive integer")
  end

  defp validate_run!(run) do
    if run.color not in ["bar", "space"],
      do: raise(ArgumentError, "run color must be 'bar' or 'space'")

    if run.modules <= 0, do: raise(ArgumentError, "run modules must be a positive integer")
  end

  # The strict portable v1 entry points are additive: legacy encoders retain
  # their historical run shape and zero-quiet-zone behavior.
  defmodule V1Error do
    defexception [:message, :error_id]

    def exception(error_id), do: %__MODULE__{message: error_id, error_id: error_id}
  end

  @max_pattern 65_567
  @max_runs 40_979
  @max_content 65_567
  @max_quiet 4_096
  @max_label 4_096
  @roles ~w(data start stop guard check inter-character-gap)
  @symbol_roles ~w(data start stop guard check)

  defp v1_fail!(id), do: raise(V1Error, id)

  defp v1_scalar_count!(value, id) when is_binary(value) do
    if String.valid?(value), do: v1_count_codepoints(value, 0), else: v1_fail!(id)
  end

  defp v1_scalar_count!(_, id), do: v1_fail!(id)

  defp v1_count_codepoints(<<>>, count), do: count
  defp v1_count_codepoints(_rest, count) when count > @max_pattern, do: count

  defp v1_count_codepoints(value, count) do
    {_scalar, rest} = String.next_codepoint(value)
    v1_count_codepoints(rest, count + 1)
  end

  defp v1_source!(label, index, role) do
    if v1_scalar_count!(label, "invalid-source-attribution") > @max_label or
         not is_integer(index) or index < -2_147_483_648 or index > 2_147_483_647 or
         role not in @roles,
       do: v1_fail!("invalid-source-attribution")
  end

  defp v1_run(color, modules, label, index, role) do
    %{
      color: color,
      modules: modules,
      source_label: :binary.copy(label),
      source_index: index,
      role: role
    }
  end

  def runs_from_binary_pattern_v1(pattern, opts \\ []) do
    count = v1_scalar_count!(pattern, "invalid-binary-token")
    if count > @max_pattern, do: v1_fail!("pattern-too-long")
    if count == 0, do: v1_fail!("empty-pattern")
    if not String.match?(pattern, ~r/\A[01]+\z/), do: v1_fail!("invalid-binary-token")
    label = Keyword.get(opts, :source_label)
    index = Keyword.get(opts, :source_index)
    role = Keyword.get(opts, :role)
    v1_source!(label, index, role)

    {runs, token, length} =
      Enum.reduce(String.codepoints(pattern), {[], nil, 0}, fn bit, {runs, token, length} ->
        if bit == token do
          {runs, token, length + 1}
        else
          next =
            if token == nil,
              do: runs,
              else: [
                v1_run(if(token == "1", do: "bar", else: "space"), length, label, index, role)
                | runs
              ]

          if length(next) >= @max_runs, do: v1_fail!("too-many-runs")
          {next, bit, 1}
        end
      end)

    Enum.reverse([
      v1_run(if(token == "1", do: "bar", else: "space"), length, label, index, role) | runs
    ])
  end

  def runs_from_width_pattern_v1(pattern, opts \\ []) do
    count = v1_scalar_count!(pattern, "invalid-width-token")
    if count > @max_pattern, do: v1_fail!("pattern-too-long")
    if count == 0, do: v1_fail!("empty-pattern")
    narrow = Keyword.get(opts, :narrow_marker, "N")
    wide = Keyword.get(opts, :wide_marker, "W")

    if v1_scalar_count!(narrow, "invalid-marker-configuration") != 1 or
         v1_scalar_count!(wide, "invalid-marker-configuration") != 1 or narrow == wide,
       do: v1_fail!("invalid-marker-configuration")

    tokens = String.codepoints(pattern)
    if Enum.any?(tokens, &(&1 not in [narrow, wide])), do: v1_fail!("invalid-width-token")
    label = Keyword.get(opts, :source_label)
    index = Keyword.get(opts, :source_index)
    role = Keyword.get(opts, :role)
    v1_source!(label, index, role)
    n_modules = Keyword.get(opts, :narrow_modules, 1)
    w_modules = Keyword.get(opts, :wide_modules, 3)

    if not (is_integer(n_modules) and n_modules > 0 and is_integer(w_modules) and w_modules > 0),
      do: v1_fail!("invalid-module-count")

    starting = Keyword.get(opts, :starting_color, "bar")
    if starting not in ["bar", "space"], do: v1_fail!("invalid-marker-configuration")
    if count > @max_runs, do: v1_fail!("too-many-runs")

    {rows, _} =
      Enum.map_reduce(Enum.with_index(tokens), 0, fn {token, position}, content ->
        modules = if token == wide, do: w_modules, else: n_modules
        if modules > @max_content - content, do: v1_fail!("content-too-wide")

        color =
          if rem(position, 2) == 0,
            do: starting,
            else: if(starting == "bar", do: "space", else: "bar")

        {v1_run(color, modules, label, index, role), content + modules}
      end)

    rows
  end

  def compute_barcode_1d_layout_v1(runs, quiet, symbols \\ nil) do
    if not is_list(runs), do: v1_fail!("invalid-source-attribution")
    if length(runs) > @max_runs, do: v1_fail!("too-many-runs")

    {copied, {content, _}} =
      Enum.map_reduce(runs, {0, nil}, fn run, {content, previous} ->
        if not is_map(run), do: v1_fail!("invalid-source-attribution")
        color = Map.get(run, :color)
        modules = Map.get(run, :modules)
        role = Map.get(run, :role)
        label = Map.get(run, :source_label)
        index = Map.get(run, :source_index)

        if color not in ["bar", "space"] or role not in @roles,
          do: v1_fail!("invalid-source-attribution")

        if not (is_integer(modules) and modules > 0), do: v1_fail!("invalid-module-count")
        v1_source!(label, index, role)
        if modules > @max_content - content, do: v1_fail!("content-too-wide")
        if color == previous, do: v1_fail!("non-alternating-runs")
        {v1_run(color, modules, label, index, role), {content + modules, color}}
      end)

    if not (is_integer(quiet) and quiet >= 1 and quiet <= @max_quiet),
      do: v1_fail!("invalid-quiet-zone")

    layouts =
      if symbols == nil,
        do: v1_infer_symbols!(copied),
        else: v1_explicit_symbols!(symbols, content)

    %{
      "leftQuietZoneModules" => quiet,
      "rightQuietZoneModules" => quiet,
      "contentModules" => content,
      "totalModules" => content + 2 * quiet,
      "symbolLayouts" => layouts
    }
  end

  defp v1_symbol(tuple, first, ending) do
    {label, index, role} = tuple

    %{
      "label" => :binary.copy(label),
      "startModule" => first,
      "endModule" => ending,
      "sourceIndex" => index,
      "role" => role
    }
  end

  defp v1_infer_symbols!(runs) do
    {reversed, active, first, cursor} =
      Enum.reduce(runs, {[], nil, 0, 0}, fn run, {rows, active, first, cursor} ->
        candidate = {run.source_label, run.source_index, run.role}

        {rows, active, first} =
          cond do
            run.role == "inter-character-gap" -> {rows, active, first}
            candidate == active -> {rows, active, first}
            active == nil -> {rows, candidate, cursor}
            true -> {[v1_symbol(active, first, cursor) | rows], candidate, cursor}
          end

        {rows, active, first, cursor + run.modules}
      end)

    reversed =
      if active == nil,
        do: reversed,
        else: [v1_symbol(active, first, cursor) | reversed]

    if length(reversed) > @max_runs, do: v1_fail!("too-many-symbols")
    Enum.reverse(reversed)
  end

  defp v1_explicit_symbols!(symbols, content) do
    if not is_list(symbols), do: v1_fail!("invalid-source-attribution")
    if length(symbols) > @max_runs, do: v1_fail!("too-many-symbols")

    {rows, width} =
      Enum.map_reduce(symbols, 0, fn symbol, cursor ->
        if not is_map(symbol), do: v1_fail!("invalid-source-attribution")
        modules = Map.get(symbol, "modules", Map.get(symbol, :modules))
        label = Map.get(symbol, "label", Map.get(symbol, :label))
        index = Map.get(symbol, "sourceIndex", Map.get(symbol, :source_index))
        role = Map.get(symbol, "role", Map.get(symbol, :role))
        if not (is_integer(modules) and modules > 0), do: v1_fail!("invalid-module-count")
        v1_source!(label, index, role)
        if role not in @symbol_roles, do: v1_fail!("invalid-source-attribution")
        if modules > @max_content - cursor, do: v1_fail!("symbol-width-mismatch")
        {v1_symbol({label, index, role}, cursor, cursor + modules), cursor + modules}
      end)

    if width != content, do: v1_fail!("symbol-width-mismatch")
    rows
  end

  defp v1_metadata!(metadata) when is_map(metadata) and map_size(metadata) <= 64 do
    {copied, _bytes} =
      Enum.reduce(metadata, {%{}, 0}, fn {key, value}, {acc, bytes} ->
        if v1_scalar_count!(key, "metadata-too-large") > 128 or
             v1_scalar_count!(value, "metadata-too-large") > 4_096 or
             byte_size(key) + byte_size(value) > 65_536 - bytes,
           do: v1_fail!("metadata-too-large")

        {Map.put(acc, :binary.copy(key), :binary.copy(value)),
         bytes + byte_size(key) + byte_size(value)}
      end)

    copied
  end

  defp v1_metadata!(_), do: v1_fail!("metadata-too-large")

  def project_barcode_1d_scene_v1(runs, quiet, options \\ %{}) do
    render = Map.get(options, :render_config, %{})

    if Map.get(options, :human_readable_text) != nil or
         (is_map(render) and Map.get(render, :include_human_readable_text, false)),
       do: v1_fail!("human-readable-text-unsupported")

    if not is_map(render), do: v1_fail!("invalid-render-config")
    module_width = Map.get(render, :module_width, 4)
    bar_height = Map.get(render, :bar_height, 120)
    foreground = Map.get(render, :foreground, "#000000")
    background = Map.get(render, :background, "#ffffff")

    if not (is_integer(module_width) and module_width in 1..8_192 and
              is_integer(bar_height) and bar_height in 1..8_192) or
         v1_scalar_count!(foreground, "invalid-render-config") > 128 or
         v1_scalar_count!(background, "invalid-render-config") > 128,
       do: v1_fail!("invalid-render-config")

    layout = compute_barcode_1d_layout_v1(runs, quiet, Map.get(options, :symbols))
    metadata = v1_metadata!(Map.get(options, :metadata, %{}))
    label = Map.get(options, :label, "1D barcode")

    if v1_scalar_count!(label, "metadata-too-large") > @max_label,
      do: v1_fail!("metadata-too-large")

    {rectangles, _cursor} =
      Enum.reduce(runs, {[], quiet}, fn run, {acc, cursor} ->
        ending = cursor + run.modules

        acc =
          if run.color == "bar" do
            rect =
              PaintInstructions.paint_rect(
                cursor * module_width,
                0,
                run.modules * module_width,
                bar_height,
                foreground,
                %{
                  "sourceLabel" => :binary.copy(run.source_label),
                  "sourceIndex" => Integer.to_string(run.source_index),
                  "role" => run.role,
                  "moduleStart" => Integer.to_string(cursor),
                  "moduleEnd" => Integer.to_string(ending)
                }
              )

            [rect | acc]
          else
            acc
          end

        {acc, ending}
      end)

    width = layout["totalModules"] * module_width

    canonical = %{
      "label" => :binary.copy(label),
      "leftQuietZoneModules" => Integer.to_string(quiet),
      "rightQuietZoneModules" => Integer.to_string(quiet),
      "contentModules" => Integer.to_string(layout["contentModules"]),
      "totalModules" => Integer.to_string(layout["totalModules"]),
      "moduleWidthPx" => Integer.to_string(module_width),
      "barHeightPx" => Integer.to_string(bar_height),
      "sceneWidthPx" => Integer.to_string(width),
      "sceneHeightPx" => Integer.to_string(bar_height),
      "symbolCount" => Integer.to_string(length(layout["symbolLayouts"]))
    }

    PaintInstructions.paint_scene(
      width,
      bar_height,
      Enum.reverse(rectangles),
      background,
      Map.merge(metadata, canonical)
    )
  end
end
