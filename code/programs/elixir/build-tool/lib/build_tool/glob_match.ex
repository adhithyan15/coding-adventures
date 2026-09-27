defmodule BuildTool.GlobMatch.PatternError do
  @moduledoc false

  defexception message: "ambiguous or descending character class in glob pattern"
end

defmodule BuildTool.GlobMatch do
  @moduledoc """
  Pure, bounded, Unicode-scalar portable-glob matching.

  Patterns and candidate paths are inert caller-supplied strings. The matcher
  never expands a pattern against the filesystem and never consults Git, the
  environment, a process, the network, or any other host authority.

  `*` matches zero or more scalars inside one path segment, `?` matches exactly
  one scalar, and a whole-segment `**` matches zero or more path segments.
  Portable character classes support literals, leading-`!` negation, ascending
  ranges, literal edge-position `-`, literal leading `]`, and unmatched `[` as
  a literal. Descending ranges and the ambiguous operators `--`, `&&`, `~~`,
  and `||` raise `BuildTool.GlobMatch.PatternError` before matching.

  Pattern parsing is linear. Segment and path matching use rolling-row dynamic
  programs, so each state in their respective rectangles is evaluated once
  and recursive suffix enumeration cannot amplify adversarial near-misses.
  """

  alias BuildTool.GlobMatch.PatternError

  @typedoc false
  @type compiled_pattern :: %{segments: [compiled_segment()]}
  @typep compiled_segment :: :globstar | {:segment, [token()]}

  @typep token ::
           :star
           | :question
           | {:literal, non_neg_integer()}
           | {:class, boolean(), MapSet.t(), list()}

  @doc """
  Reports whether a relative path matches one portable glob pattern.
  """
  @spec match_path?(String.t(), String.t()) :: boolean()
  def match_path?(pattern, path) do
    match_path_with_stats(pattern, path).matched
  end

  @doc false
  @spec match_path_with_stats(String.t(), String.t()) :: map()
  def match_path_with_stats(pattern, path) do
    {compiled, parser_steps} = compile_pattern_with_stats!(pattern)
    {matched, path_states, segment_states} = match_compiled_with_stats(compiled, path)

    %{
      matched: matched,
      parser_steps: parser_steps,
      path_states: path_states,
      segment_states: segment_states
    }
  end

  @doc """
  Validates one portable glob pattern without matching a candidate path.
  """
  @spec validate_pattern!(String.t()) :: :ok
  def validate_pattern!(pattern) do
    {_compiled, _parser_steps} = compile_pattern_with_stats!(pattern)
    :ok
  end

  @doc false
  @spec validate_pattern_with_stats!(String.t()) :: map()
  def validate_pattern_with_stats!(pattern) do
    {_compiled, parser_steps} = compile_pattern_with_stats!(pattern)
    %{parser_steps: parser_steps}
  end

  @doc false
  @spec compile_patterns!([String.t()]) :: [compiled_pattern()]
  def compile_patterns!(patterns) when is_list(patterns) do
    Enum.map(patterns, fn pattern ->
      {compiled, _parser_steps} = compile_pattern_with_stats!(pattern)
      compiled
    end)
  end

  @doc false
  @spec match_any_compiled_path?([compiled_pattern()], String.t()) :: boolean()
  def match_any_compiled_path?(compiled_patterns, path) do
    Enum.any?(compiled_patterns, &match_compiled_path?(&1, path))
  end

  @doc false
  @spec match_compiled_path?(compiled_pattern(), String.t()) :: boolean()
  def match_compiled_path?(compiled, path) do
    {matched, _path_states, _segment_states} = match_compiled_with_stats(compiled, path)
    matched
  end

  @doc false
  @spec split_path(String.t()) :: [String.t()]
  def split_path(""), do: []

  def split_path(path) do
    path
    |> String.split("/")
    |> Enum.reject(&(&1 == ""))
  end

  defp compile_pattern_with_stats!(pattern) do
    pattern = String.trim_trailing(pattern, "/")

    {segments, parser_steps} =
      pattern
      |> split_path()
      |> Enum.reduce({[], 0}, fn
        "**", {[:globstar | _] = segments, steps} ->
          {segments, steps + 2}

        "**", {segments, steps} ->
          {[:globstar | segments], steps + 2}

        segment, {segments, steps} ->
          {tokens, segment_steps} = compile_segment!(segment)
          {[{:segment, tokens} | segments], steps + segment_steps}
      end)

    {%{segments: Enum.reverse(segments)}, parser_steps}
  end

  defp compile_segment!(segment) do
    scalars = String.to_charlist(segment)
    scalar_tuple = List.to_tuple(scalars)
    length = tuple_size(scalar_tuple)
    next_close = next_close_indices(scalars)

    {tokens, parse_steps} =
      compile_tokens(scalar_tuple, next_close, length, 0, [], length)

    {Enum.reverse(tokens), length + parse_steps}
  end

  defp next_close_indices(scalars) do
    {indices, _next} =
      scalars
      |> Enum.with_index()
      |> Enum.reverse()
      |> Enum.reduce({[], nil}, fn {scalar, index}, {indices, next} ->
        next = if scalar == ?], do: index, else: next
        {[next | indices], next}
      end)

    List.to_tuple(indices)
  end

  defp compile_tokens(_scalars, _next_close, length, length, tokens, steps),
    do: {tokens, steps}

  defp compile_tokens(scalars, next_close, length, index, tokens, steps) do
    scalar = elem(scalars, index)

    cond do
      scalar == ?* ->
        tokens = if List.first(tokens) == :star, do: tokens, else: [:star | tokens]
        compile_tokens(scalars, next_close, length, index + 1, tokens, steps + 1)

      scalar == ?? ->
        compile_tokens(scalars, next_close, length, index + 1, [:question | tokens], steps + 1)

      scalar == ?[ ->
        case character_class(scalars, next_close, length, index) do
          :unmatched ->
            compile_tokens(
              scalars,
              next_close,
              length,
              index + 1,
              [{:literal, ?[} | tokens],
              steps + 1
            )

          {:ok, class_token, close_index, class_steps} ->
            compile_tokens(
              scalars,
              next_close,
              length,
              close_index + 1,
              [class_token | tokens],
              steps + class_steps
            )
        end

      true ->
        compile_tokens(
          scalars,
          next_close,
          length,
          index + 1,
          [{:literal, scalar} | tokens],
          steps + 1
        )
    end
  end

  defp character_class(scalars, next_close, length, open_index) do
    raw_start = open_index + 1

    {negated, body_start} =
      if raw_start < length and elem(scalars, raw_start) == ?! do
        {true, raw_start + 1}
      else
        {false, raw_start}
      end

    close_search =
      if body_start < length and elem(scalars, body_start) == ?] do
        body_start + 1
      else
        body_start
      end

    close_index =
      if close_search < length do
        elem(next_close, close_search)
      end

    if is_nil(close_index) do
      :unmatched
    else
      body = tuple_slice(scalars, body_start, close_index)
      {literals, ranges} = parse_class_body!(body)
      {:ok, {:class, negated, literals, ranges}, close_index, length(body) + 2}
    end
  end

  defp tuple_slice(_tuple, start, stop) when start >= stop, do: []

  defp tuple_slice(tuple, start, stop) do
    for index <- start..(stop - 1), do: elem(tuple, index)
  end

  defp parse_class_body!(body) do
    if ambiguous_class_operator?(body) do
      raise PatternError
    end

    parse_class_members(body, MapSet.new(), [])
  end

  defp ambiguous_class_operator?([left, right | rest]) do
    (left == right and left in [?-, ?&, ?~, ?|]) or
      ambiguous_class_operator?([right | rest])
  end

  defp ambiguous_class_operator?(_), do: false

  defp parse_class_members([low, ?-, high | rest], literals, ranges) do
    if low > high do
      raise PatternError
    end

    parse_class_members(rest, literals, [{low, high} | ranges])
  end

  defp parse_class_members([literal | rest], literals, ranges) do
    parse_class_members(rest, MapSet.put(literals, literal), ranges)
  end

  defp parse_class_members([], literals, ranges), do: {literals, Enum.reverse(ranges)}

  defp match_compiled_with_stats(%{segments: segments}, path) do
    path_parts =
      path
      |> String.trim_trailing("/")
      |> split_path()
      |> Enum.map(&String.to_charlist/1)
      |> List.to_tuple()

    path_count = tuple_size(path_parts)
    base_row = false_row_with_terminal_true(path_count)

    {result_row, segment_states} =
      segments
      |> Enum.reverse()
      |> Enum.reduce({base_row, 0}, fn
        :globstar, {next_row, states} ->
          {globstar_row(next_row, path_count), states}

        {:segment, tokens}, {next_row, states} ->
          {row, added_states} = segment_path_row(tokens, next_row, path_parts, path_count)
          {row, states + added_states}
      end)

    path_states = (length(segments) + 1) * (path_count + 1)
    {elem(result_row, 0), path_states, segment_states}
  end

  defp false_row_with_terminal_true(last_index) do
    List.duplicate(false, last_index) |> Kernel.++([true]) |> List.to_tuple()
  end

  defp globstar_row(next_row, path_count) do
    {_right, row} =
      Enum.reduce(path_count..0//-1, {false, []}, fn index, {right, row} ->
        value = elem(next_row, index) or (index < path_count and right)
        {value, [value | row]}
      end)

    List.to_tuple(row)
  end

  defp segment_path_row(tokens, next_row, path_parts, path_count) do
    {row, states} =
      Enum.reduce(path_count..0//-1, {[], 0}, fn
        index, {row, states} when index == path_count ->
          {[false | row], states}

        index, {row, states} ->
          {segment_match, segment_states} =
            match_segment_with_stats(tokens, elem(path_parts, index))

          value = segment_match and elem(next_row, index + 1)
          {[value | row], states + segment_states}
      end)

    {List.to_tuple(row), states}
  end

  defp match_segment_with_stats(tokens, path_scalars) do
    path_count = length(path_scalars)
    path_tuple = List.to_tuple(path_scalars)
    base_row = false_row_with_terminal_true(path_count)

    result_row =
      tokens
      |> Enum.reverse()
      |> Enum.reduce(base_row, fn
        :star, next_row -> star_scalar_row(next_row, path_count)
        token, next_row -> scalar_token_row(token, next_row, path_tuple, path_count)
      end)

    states = (length(tokens) + 1) * (path_count + 1)
    {elem(result_row, 0), states}
  end

  defp star_scalar_row(next_row, path_count) do
    {_right, row} =
      Enum.reduce(path_count..0//-1, {false, []}, fn index, {right, row} ->
        value = elem(next_row, index) or (index < path_count and right)
        {value, [value | row]}
      end)

    List.to_tuple(row)
  end

  defp scalar_token_row(token, next_row, path_tuple, path_count) do
    row =
      Enum.reduce(path_count..0//-1, [], fn
        index, row when index == path_count ->
          [false | row]

        index, row ->
          value = token_matches?(token, elem(path_tuple, index)) and elem(next_row, index + 1)
          [value | row]
      end)

    List.to_tuple(row)
  end

  defp token_matches?(:question, _scalar), do: true
  defp token_matches?({:literal, scalar}, scalar), do: true
  defp token_matches?({:literal, _expected}, _actual), do: false

  defp token_matches?({:class, negated, literals, ranges}, scalar) do
    included =
      MapSet.member?(literals, scalar) or
        Enum.any?(ranges, fn {low, high} -> scalar >= low and scalar <= high end)

    if negated, do: not included, else: included
  end
end
