defmodule CodingAdventures.CliBuilder.TokenClassifier do
  @moduledoc """
  Classifies a single argv token into a typed token event.

  ## The Token Classification DFA

  The spec (§5.1) describes a DFA that reads one argv token character-by-character
  and emits a typed token event. This module implements that DFA as a set of
  Elixir pattern-matching functions. Rather than building a literal
  `CodingAdventures.StateMachine.DFA` struct (which would require us to enumerate
  all possible flag characters as an alphabet), we implement the rules directly
  in code — the logic is equivalent to a DFA transition table, just expressed in
  a form that is easier to read and maintain.

  ## Token Types (§5.1)

  | Token type | Pattern |
  |---|---|
  | `:end_of_flags` | Exactly `--` |
  | `{:long_flag, name}` | `--name` (no `=`) |
  | `{:long_flag_with_value, name, value}` | `--name=value` |
  | `{:single_dash_long, name}` | `-name` matching a `single_dash_long` flag |
  | `{:short_flag, char}` | `-x` where `x` is a single declared short flag |
  | `{:short_flag_with_value, char, value}` | `-xVALUE` where `x` is non-boolean |
  | `{:stacked_flags, chars}` | `-xyz` of boolean short flags |
  | `{:positional, value}` | Any non-flag-looking token |
  | `{:unknown_flag, token}` | Token that looks like a flag but matches nothing |

  ## Longest-Match-First Disambiguation (§5.2)

  For tokens starting with `-` (but not `--`), rules are tried in order:
  1. Single-dash-long match
  2. Short flag match (with possible stacking / inline value)
  3. No match → unknown flag

  ## Usage

      flags = spec["flags"] ++ spec["global_flags"]
      result = TokenClassifier.classify("--output=foo.txt", flags)
      # => {:long_flag_with_value, "output", "foo.txt"}
  """

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Classify a single argv token given the set of flags currently in scope.

  `active_flags` is a list of normalised flag definition maps (as produced by
  `SpecLoader`). Each map must have the keys `"id"`, `"short"`, `"long"`,
  `"single_dash_long"`, and `"type"`.

  Returns one of:

  - `:end_of_flags`
  - `{:long_flag, name}`
  - `{:long_flag_with_value, name, value}`
  - `{:single_dash_long, name}`
  - `{:short_flag, char}`
  - `{:short_flag_with_value, char, value}`
  - `{:stacked_flags, [char]}`
  - `{:positional, value}`
  - `{:unknown_flag, token}`

  ## Examples

      iex> alias CodingAdventures.CliBuilder.TokenClassifier
      iex> flags = [%{"id" => "verbose", "short" => "v", "long" => "verbose",
      ...>            "single_dash_long" => nil, "type" => "boolean"}]
      iex> TokenClassifier.classify("--verbose", flags)
      {:long_flag, "verbose"}
      iex> TokenClassifier.classify("-v", flags)
      {:short_flag, "v"}
      iex> TokenClassifier.classify("hello", flags)
      {:positional, "hello"}
  """
  @spec classify(String.t(), [map()]) ::
          :end_of_flags
          | {:long_flag, String.t()}
          | {:long_flag_with_value, String.t(), String.t()}
          | {:single_dash_long, String.t()}
          | {:short_flag, String.t()}
          | {:short_flag_with_value, String.t(), String.t()}
          | {:stacked_flags, [String.t()]}
          | {:positional, String.t()}
          | {:unknown_flag, String.t()}
  def classify(token, active_flags) do
    cond do
      # Exact "--": end-of-flags marker.
      token == "--" ->
        :end_of_flags

      # Long flags: "--name" or "--name=value"
      String.starts_with?(token, "--") ->
        classify_long(token, active_flags)

      # Single "-" with no following chars: conventional stdin/stdout token.
      token == "-" ->
        {:positional, "-"}

      # Short flags: "-x..." (single dash followed by one or more chars)
      String.starts_with?(token, "-") ->
        classify_short(token, active_flags)

      # Anything else is positional.
      true ->
        {:positional, token}
    end
  end

  # ---------------------------------------------------------------------------
  # Long flag classification
  # ---------------------------------------------------------------------------

  # Handles tokens that begin with "--".
  # The substring after "--" may contain "=" separating the flag name from its value.
  defp classify_long(token, _active_flags) do
    rest = String.slice(token, 2, String.length(token))

    case String.split(rest, "=", parts: 2) do
      [name, value] ->
        # "--name=value" form
        {:long_flag_with_value, name, value}

      [name] ->
        # "--name" form (value, if needed, is the next token)
        {:long_flag, name}
    end
  end

  # ---------------------------------------------------------------------------
  # Short / single-dash-long classification
  # ---------------------------------------------------------------------------

  # Handles tokens that begin with a single "-".
  # The spec (§5.2) applies rules in order:
  #   Rule 1 — single-dash-long: the whole substring after "-" matches a SDL flag
  #   Rule 2 — short flag: token[1] matches a short flag
  #   Rule 3 — stacked flags: walk each character
  #   Rule 4 — no match → :unknown_flag
  defp classify_short(token, active_flags) do
    # The part after the leading "-"
    rest = String.slice(token, 1, String.length(token))

    # Build lookup structures once.
    sdl_map = build_sdl_map(active_flags)
    short_map = build_short_map(active_flags)

    cond do
      # Rule 1 — single-dash-long exact match
      Map.has_key?(sdl_map, rest) ->
        {:single_dash_long, rest}

      # Rule 2 — first char is a known short flag
      String.length(rest) >= 1 and Map.has_key?(short_map, String.first(rest)) ->
        char = String.first(rest)
        flag = Map.fetch!(short_map, char)
        remainder = String.slice(rest, 1, String.length(rest))

        cond do
          remainder == "" ->
            # "-x" with nothing left
            {:short_flag, char}

          flag["type"] in ["boolean", "count"] ->
            # Boolean and count flags consume no value; try to stack the
            # remainder characters.  Count flags behave identically to boolean
            # flags during tokenisation — each occurrence simply increments
            # the counter in the parser.
            classify_stacked(char, remainder, short_map, sdl_map)

          true ->
            # Non-boolean: everything after the flag char is the inline value.
            {:short_flag_with_value, char, remainder}
        end

      # Rule 4 — no match → unknown flag
      true ->
        {:unknown_flag, token}
    end
  end

  # ---------------------------------------------------------------------------
  # Stacked flag classification (Rule 3)
  # ---------------------------------------------------------------------------

  # We have confirmed the first char is a boolean short flag.
  # Now walk the remainder to build a full STACKED_FLAGS list.
  #
  # All chars except possibly the last must be boolean flags.
  # If a non-boolean flag is encountered and it is the last char, it expects
  # its value from the next token — treated as stacked with the others.
  # If a char is unknown, we emit :unknown_flag for the whole token.
  defp classify_stacked(first_char, rest, short_map, _sdl_map) do
    chars = String.graphemes(rest)
    result = walk_stack(chars, short_map, [first_char])

    case result do
      {:ok, stack} -> {:stacked_flags, stack}
      {:unknown, _} -> {:unknown_flag, "-" <> first_char <> rest}
    end
  end

  # Walk each character in the remainder of a potential stacked flag sequence.
  # Returns {:ok, [chars]} on success or {:unknown, char} on the first unknown char.
  defp walk_stack([], _short_map, acc), do: {:ok, Enum.reverse(acc)}

  defp walk_stack([char | rest], short_map, acc) do
    case Map.get(short_map, char) do
      nil ->
        # Unknown flag character in stack.
        {:unknown, char}

      flag ->
        if flag["type"] in ["boolean", "count"] or rest == [] do
          # Boolean and count flags consume no value and are always safe to
          # stack.  The last flag in the stack (which may consume the next
          # token as its value) is also OK regardless of type.
          walk_stack(rest, short_map, [char | acc])
        else
          # Non-boolean flag in the middle of a stack — the remaining chars
          # would be ambiguous. Treat as unknown.
          {:unknown, char}
        end
    end
  end

  # ---------------------------------------------------------------------------
  # Lookup map builders
  # ---------------------------------------------------------------------------

  # Build a map from single_dash_long name -> flag def.
  # Only includes flags that actually have a single_dash_long value.
  defp build_sdl_map(flags) do
    flags
    |> Enum.filter(fn f -> f["single_dash_long"] != nil end)
    |> Map.new(fn f -> {f["single_dash_long"], f} end)
  end

  # Build a map from short character -> flag def.
  # Only includes flags that have a `short` field.
  defp build_short_map(flags) do
    flags
    |> Enum.filter(fn f -> f["short"] != nil end)
    |> Map.new(fn f -> {f["short"], f} end)
  end
end
