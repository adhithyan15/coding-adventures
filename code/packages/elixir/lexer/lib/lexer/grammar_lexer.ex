defmodule CodingAdventures.Lexer.GrammarLexer do
  @moduledoc """
  Grammar-Driven Lexer — Tokenization from .tokens Files
  =======================================================

  This module is the Elixir port of the Python `GrammarLexer`. Instead of
  hardcoding which characters map to which tokens, it reads token definitions
  from a `TokenGrammar` (parsed from a `.tokens` file) and uses those
  definitions to drive tokenization at runtime.

  ## How It Works

  1. **Compile** each token definition's pattern into an Elixir `Regex`.
     Literal patterns are escaped so that characters like `+` and `*` are
     treated as literal characters, not regex operators.

  2. **At each position** in the source code, try each compiled pattern in
     order (first match wins). This is the "priority" mechanism — if two
     patterns could match at the same position, the one that appears first
     in the `.tokens` file wins.

  3. **Emit a Token** with the matched type and value.

  ## State Threading

  Since Elixir is immutable, the lexer state (position, line, column) is
  threaded through every function as a `%State{}` struct. Each function
  returns `{result, updated_state}` tuples, passing the ball forward.
  """

  alias CodingAdventures.Lexer.Token
  alias CodingAdventures.GrammarTools.TokenGrammar

  # -- Lexer state (threaded through all functions) ---------------------------

  defmodule State do
    @moduledoc false
    defstruct [
      :source,
      :patterns,
      :skip_patterns,
      :keyword_set,
      :reserved_set,
      :alias_map,
      :has_skip_patterns,
      pos: 0,
      line: 1,
      column: 1
    ]
  end

  # -- Public API -------------------------------------------------------------

  @doc """
  Tokenize source code using a `TokenGrammar`.

  Compiles the grammar's patterns, then walks the source character by
  character, trying each pattern at each position (first match wins).

  Returns `{:ok, tokens}` on success, `{:error, message}` on failure.
  The token list always ends with an EOF token.
  """
  @spec tokenize(String.t(), TokenGrammar.t()) :: {:ok, [Token.t()]} | {:error, String.t()}
  def tokenize(source, %TokenGrammar{} = grammar) do
    state = init_state(source, grammar)
    tokenize_standard(state, [])
  end

  # -- Initialization ---------------------------------------------------------

  defp init_state(source, grammar) do
    # Compile token patterns: [{name, compiled_regex}, ...]
    # Order matters — first match wins, matching Lex/Flex behavior.
    patterns =
      Enum.map(grammar.definitions, fn defn ->
        regex =
          if defn.is_regex do
            Regex.compile!("\\A" <> defn.pattern)
          else
            Regex.compile!("\\A" <> Regex.escape(defn.pattern))
          end

        {defn.name, regex}
      end)

    # Compile skip patterns (whitespace, comments consumed silently).
    skip_patterns =
      Enum.map(grammar.skip_definitions, fn defn ->
        if defn.is_regex do
          Regex.compile!("\\A" <> defn.pattern)
        else
          Regex.compile!("\\A" <> Regex.escape(defn.pattern))
        end
      end)

    # Build alias map: definition name → alias name.
    alias_map =
      grammar.definitions
      |> Enum.filter(& &1.alias)
      |> Map.new(fn defn -> {defn.name, defn.alias} end)

    %State{
      source: source,
      patterns: patterns,
      skip_patterns: skip_patterns,
      keyword_set: MapSet.new(grammar.keywords),
      reserved_set: MapSet.new(grammar.reserved_keywords),
      alias_map: alias_map,
      has_skip_patterns: length(grammar.skip_definitions) > 0
    }
  end

  # -- Standard (non-indentation) tokenization --------------------------------
  #
  # The algorithm:
  # 1. While there are characters left:
  #    a. If skip patterns exist, try them (consume silently).
  #    b. If no skip patterns, use default whitespace skip.
  #    c. If the current character is a newline, emit NEWLINE.
  #    d. Try each token pattern (first match wins).
  #    e. If nothing matches, return error.
  # 2. Append EOF.

  defp tokenize_standard(%State{pos: pos, source: source} = state, tokens)
       when pos >= byte_size(source) do
    eof = %Token{type: "EOF", value: "", line: state.line, column: state.column}
    {:ok, Enum.reverse([eof | tokens])}
  end

  defp tokenize_standard(%State{} = state, tokens) do
    remaining = binary_part(state.source, state.pos, byte_size(state.source) - state.pos)

    cond do
      # Try skip patterns first (grammar-defined whitespace/comments)
      state.has_skip_patterns ->
        case try_skip(remaining, state) do
          {:matched, new_state} ->
            tokenize_standard(new_state, tokens)

          :no_match ->
            # After skip fails, try newline then token patterns
            case try_newline_or_token(remaining, state, tokens) do
              {:continue, new_state, new_tokens} ->
                tokenize_standard(new_state, new_tokens)

              {:error, msg} ->
                {:error, msg}
            end
        end

      # No skip patterns — use default whitespace skipping
      true ->
        ch = binary_part(remaining, 0, 1)

        if ch in [" ", "\t", "\r"] do
          tokenize_standard(advance(state), tokens)
        else
          case try_newline_or_token(remaining, state, tokens) do
            {:continue, new_state, new_tokens} ->
              tokenize_standard(new_state, new_tokens)

            {:error, msg} ->
              {:error, msg}
          end
        end
    end
  end

  defp try_newline_or_token(remaining, state, tokens) do
    ch = binary_part(remaining, 0, 1)

    if ch == "\n" do
      token = %Token{type: "NEWLINE", value: "\\n", line: state.line, column: state.column}
      {:continue, advance(state), [token | tokens]}
    else
      case try_match_token(remaining, state) do
        {:matched, token, new_state} ->
          {:continue, new_state, [token | tokens]}

        {:error, msg} ->
          {:error, msg}

        :no_match ->
          {:error,
           "Line #{state.line}, column #{state.column}: Unexpected character: #{inspect(ch)}"}
      end
    end
  end

  # -- Skip pattern matching --------------------------------------------------

  defp try_skip(remaining, state) do
    Enum.reduce_while(state.skip_patterns, :no_match, fn pattern, _acc ->
      case Regex.run(pattern, remaining, capture: :first) do
        [match] ->
          new_state = advance_by(state, match)
          {:halt, {:matched, new_state}}

        _ ->
          {:cont, :no_match}
      end
    end)
  end

  # -- Token pattern matching -------------------------------------------------

  defp try_match_token(remaining, state) do
    Enum.reduce_while(state.patterns, :no_match, fn {token_name, pattern}, _acc ->
      case Regex.run(pattern, remaining, capture: :first) do
        [match] ->
          start_line = state.line
          start_column = state.column

          # Resolve token type (keyword detection, alias, etc.)
          case resolve_token_type(token_name, match, state) do
            {:ok, token_type} ->
              # Handle STRING tokens: strip quotes and process escapes
              effective_name = Map.get(state.alias_map, token_name, token_name)

              value =
                if effective_name == "STRING" or token_name == "STRING" do
                  match
                  |> String.slice(1..-2//1)
                  |> process_escapes()
                else
                  match
                end

              token = %Token{
                type: token_type,
                value: value,
                line: start_line,
                column: start_column
              }

              new_state = advance_by(state, match)
              {:halt, {:matched, token, new_state}}

            {:error, msg} ->
              {:halt, {:error, msg}}
          end

        _ ->
          {:cont, :no_match}
      end
    end)
    |> case do
      {:error, msg} -> {:error, msg}
      other -> other
    end
  end

  # -- Token type resolution --------------------------------------------------
  #
  # Resolution order:
  # 1. Reserved keyword check → error
  # 2. Keyword detection → "KEYWORD"
  # 3. Alias resolution → alias name
  # 4. Fallback → token name as string

  defp resolve_token_type(token_name, value, state) do
    effective_name = Map.get(state.alias_map, token_name, token_name)

    cond do
      effective_name == "NAME" and MapSet.member?(state.reserved_set, value) ->
        {:error,
         "Line #{state.line}, column #{state.column}: " <>
           "Reserved keyword '#{value}' cannot be used as an identifier"}

      effective_name == "NAME" and MapSet.member?(state.keyword_set, value) ->
        {:ok, "KEYWORD"}

      true ->
        {:ok, effective_name}
    end
  end

  # -- Escape processing for string tokens ------------------------------------
  #
  # Handles the same escape sequences as the Python GrammarLexer:
  # \n, \t, \r, \b, \f, \\, \", \/, and \uXXXX unicode escapes.

  @doc false
  def process_escapes(s), do: process_escapes(s, 0, [])

  defp process_escapes(s, i, acc) when i >= byte_size(s) do
    acc |> Enum.reverse() |> IO.iodata_to_binary()
  end

  defp process_escapes(s, i, acc) do
    ch = binary_part(s, i, 1)

    if ch == "\\" and i + 1 < byte_size(s) do
      next = binary_part(s, i + 1, 1)

      escape_map = %{
        "n" => "\n",
        "t" => "\t",
        "r" => "\r",
        "b" => "\b",
        "f" => "\f",
        "\\" => "\\",
        "\"" => "\"",
        "/" => "/"
      }

      cond do
        Map.has_key?(escape_map, next) ->
          process_escapes(s, i + 2, [escape_map[next] | acc])

        next == "u" and i + 5 < byte_size(s) ->
          hex_str = binary_part(s, i + 2, 4)

          if Regex.match?(~r/\A[0-9a-fA-F]{4}\z/, hex_str) do
            codepoint = String.to_integer(hex_str, 16)
            process_escapes(s, i + 6, [<<codepoint::utf8>> | acc])
          else
            process_escapes(s, i + 2, [next | acc])
          end

        true ->
          process_escapes(s, i + 2, [next | acc])
      end
    else
      process_escapes(s, i + 1, [ch | acc])
    end
  end

  # -- Position advancement ---------------------------------------------------

  defp advance(%State{pos: pos, source: source} = state) when pos < byte_size(source) do
    ch = binary_part(source, pos, 1)

    if ch == "\n" do
      %{state | pos: pos + 1, line: state.line + 1, column: 1}
    else
      %{state | pos: pos + 1, column: state.column + 1}
    end
  end

  defp advance(state), do: state

  defp advance_by(state, <<>>), do: state

  defp advance_by(state, <<_ch::utf8, rest::binary>>) do
    advance_by(advance(state), rest)
  end
end
