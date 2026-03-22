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

  ## Pattern Groups & Callbacks — Functional Style
  ------------------------------------------------

  The Python GrammarLexer uses a mutable `LexerContext` class with methods
  like `push_group()`, `pop_group()`, `emit()`, and `suppress()`. In
  Elixir, we take a **functional approach** instead:

  - The **callback** is a function that receives a token and a read-only
    `LexerContext` struct, and returns a list of **action tuples**.
  - The tokenizer applies those actions after the callback returns.
  - No mutation, no side effects — just data in, data out.

  ### Action tuples:

      {:push_group, group_name}   — push a pattern group onto the stack
      :pop_group                  — pop the current group (no-op at bottom)
      {:emit, token}              — inject a synthetic token after the current one
      :suppress                   — suppress the current token from output
      {:set_skip_enabled, bool}   — toggle skip pattern processing

  ### Example — XML lexer callback:

      callback = fn token, _ctx ->
        case token.type do
          "OPEN_TAG" -> [{:push_group, "tag"}]
          "TAG_CLOSE" -> [:pop_group]
          _ -> []
        end
      end

      {:ok, tokens} = GrammarLexer.tokenize(source, grammar, on_token: callback)
  """

  alias CodingAdventures.Lexer.Token
  alias CodingAdventures.GrammarTools.TokenGrammar

  # ---------------------------------------------------------------------------
  # LexerContext — Read-Only Callback Interface
  # ---------------------------------------------------------------------------

  defmodule LexerContext do
    @moduledoc """
    Read-only context passed to on-token callbacks.

    Unlike the Python `LexerContext` which has mutating methods, the Elixir
    version is a simple struct with read-only information. The callback
    returns a list of action tuples instead of calling methods.

    ## Fields

    - `active_group` — the name of the currently active pattern group
    - `group_stack_depth` — the depth of the group stack (always >= 1)
    - `source` — the full source string being tokenized
    - `pos_after_token` — the position in source immediately after the
      current token (useful for peeking ahead)

    ## Peeking Ahead

    To peek at characters after the current token, use `peek/2` or
    `peek_str/2`:

        # Peek at the next character after the token:
        LexerContext.peek(ctx, 1)  # => "x" or ""

        # Peek at the next 5 characters:
        LexerContext.peek_str(ctx, 5)  # => "hello"

    ## Available Groups

    The `available_groups` field lists all group names defined in the
    grammar. Use this to validate group names before returning a
    `{:push_group, name}` action.
    """

    defstruct [
      :active_group,
      :group_stack_depth,
      :source,
      :pos_after_token,
      :available_groups
    ]

    @type t :: %__MODULE__{
            active_group: String.t(),
            group_stack_depth: pos_integer(),
            source: String.t(),
            pos_after_token: non_neg_integer(),
            available_groups: [String.t()]
          }

    @doc """
    Peek at a source character past the current token.

    The `offset` is 1-based: offset 1 means the character immediately
    after the token, offset 2 means one further, etc.

    Returns the character as a single-character string, or `""` if the
    offset is past the end of the source.

    ## Examples

        iex> ctx = %LexerContext{source: "hello", pos_after_token: 3}
        iex> LexerContext.peek(ctx, 1)
        "l"
        iex> LexerContext.peek(ctx, 3)
        ""
    """
    @spec peek(t(), pos_integer()) :: String.t()
    def peek(%__MODULE__{source: source, pos_after_token: pos}, offset) do
      idx = pos + offset - 1

      if idx >= 0 and idx < byte_size(source) do
        binary_part(source, idx, 1)
      else
        ""
      end
    end

    @doc """
    Peek at the next `length` characters past the current token.

    Returns a substring of up to `length` characters starting from the
    position immediately after the current token. If fewer characters
    remain, returns whatever is available.

    ## Examples

        iex> ctx = %LexerContext{source: "hello world", pos_after_token: 5}
        iex> LexerContext.peek_str(ctx, 6)
        " world"
    """
    @spec peek_str(t(), non_neg_integer()) :: String.t()
    def peek_str(%__MODULE__{source: source, pos_after_token: pos}, length) do
      available = byte_size(source) - pos
      take = min(length, max(available, 0))

      if take > 0 do
        binary_part(source, pos, take)
      else
        ""
      end
    end
  end

  # ---------------------------------------------------------------------------
  # Action Types
  # ---------------------------------------------------------------------------
  #
  # Callbacks return a list of these action tuples. The tokenizer applies
  # them in order after the callback returns.
  #
  # @type action :: {:push_group, String.t()}
  #              | :pop_group
  #              | {:emit, Token.t()}
  #              | :suppress
  #              | {:set_skip_enabled, boolean()}
  #
  # @type callback :: (Token.t(), LexerContext.t() -> [action()])

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
      :escape_mode,
      # -- Pattern groups ---
      # Maps group name → [{name, compiled_regex}, ...]
      # The "default" group always exists and uses the top-level definitions.
      :group_patterns,
      # The group stack. Bottom is always "default". Top is the active group.
      group_stack: ["default"],
      # -- Callback ---
      # Optional function (Token, LexerContext) -> [action]
      on_token: nil,
      # Whether skip patterns are active. Callbacks can toggle this.
      skip_enabled: true,
      # -- Position tracking ---
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

  ## Options

  - `:on_token` — an optional callback function that fires after each
    token match, before emission. The callback receives a `Token` and a
    `LexerContext`, and returns a list of action tuples. See the module
    documentation for the action types.

  ## Examples

      # Without callback (original behavior):
      {:ok, tokens} = GrammarLexer.tokenize("x + 1", grammar)

      # With callback for group switching:
      callback = fn token, _ctx ->
        case token.type do
          "OPEN_TAG" -> [{:push_group, "tag"}]
          "TAG_CLOSE" -> [:pop_group]
          _ -> []
        end
      end
      {:ok, tokens} = GrammarLexer.tokenize(source, grammar, on_token: callback)
  """
  @spec tokenize(String.t(), TokenGrammar.t(), keyword()) ::
          {:ok, [Token.t()]} | {:error, String.t()}
  def tokenize(source, grammar, opts \\ [])

  def tokenize(source, %TokenGrammar{} = grammar, opts) do
    on_token = Keyword.get(opts, :on_token, nil)
    state = init_state(source, grammar, on_token)
    tokenize_standard(state, [])
  end

  # -- Initialization ---------------------------------------------------------

  defp init_state(source, grammar, on_token) do
    # Compile token patterns: [{name, compiled_regex}, ...]
    # Order matters — first match wins, matching Lex/Flex behavior.
    patterns = compile_definitions(grammar.definitions)

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
    # Includes aliases from both top-level and group definitions.
    alias_map =
      grammar.definitions
      |> Enum.filter(& &1.alias)
      |> Map.new(fn defn -> {defn.name, defn.alias} end)

    # Also collect aliases from group definitions.
    group_alias_map =
      grammar.groups
      |> Map.values()
      |> Enum.flat_map(& &1.definitions)
      |> Enum.filter(& &1.alias)
      |> Map.new(fn defn -> {defn.name, defn.alias} end)

    merged_alias_map = Map.merge(alias_map, group_alias_map)

    # -- Pattern groups ---
    # Compile per-group patterns. The "default" group uses the top-level
    # definitions. Named groups use their own definitions.
    group_patterns =
      grammar.groups
      |> Enum.reduce(%{"default" => patterns}, fn {group_name, group}, acc ->
        compiled = compile_definitions(group.definitions)
        Map.put(acc, group_name, compiled)
      end)

    %State{
      source: source,
      patterns: patterns,
      skip_patterns: skip_patterns,
      keyword_set: MapSet.new(grammar.keywords),
      reserved_set: MapSet.new(grammar.reserved_keywords),
      alias_map: merged_alias_map,
      has_skip_patterns: length(grammar.skip_definitions) > 0,
      escape_mode: grammar.escape_mode,
      group_patterns: group_patterns,
      group_stack: ["default"],
      on_token: on_token,
      skip_enabled: true
    }
  end

  # Compile a list of token definitions into [{name, compiled_regex}, ...]
  defp compile_definitions(definitions) do
    Enum.map(definitions, fn defn ->
      regex =
        if defn.is_regex do
          Regex.compile!("\\A" <> defn.pattern)
        else
          Regex.compile!("\\A" <> Regex.escape(defn.pattern))
        end

      {defn.name, regex}
    end)
  end

  # -- Standard (non-indentation) tokenization --------------------------------
  #
  # The algorithm:
  # 1. While there are characters left:
  #    a. If skip patterns exist and skip is enabled, try them.
  #    b. If no skip patterns, use default whitespace skip.
  #    c. If the current character is a newline, emit NEWLINE.
  #    d. Try active group's token patterns (first match wins).
  #    e. If callback registered, invoke it and process actions.
  #    f. If nothing matches, return error.
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
      # Only when skip is enabled — callbacks can disable skip for groups
      # where whitespace is significant (e.g., CDATA, raw text).
      state.has_skip_patterns and state.skip_enabled ->
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

      # Skip patterns exist but skip is disabled — go straight to tokens
      state.has_skip_patterns ->
        case try_newline_or_token(remaining, state, tokens) do
          {:continue, new_state, new_tokens} ->
            tokenize_standard(new_state, new_tokens)

          {:error, msg} ->
            {:error, msg}
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
      # Use the active group's patterns (top of the group stack).
      # When no groups are defined, this is always "default", preserving
      # backward compatibility with grammars that don't use groups.
      active_group = List.first(state.group_stack)
      group_pats = Map.get(state.group_patterns, active_group, state.patterns)

      case try_match_token_in_group(remaining, state, group_pats) do
        {:matched, token, new_state} ->
          # If a callback is registered, invoke it and process the
          # returned actions. The callback receives a read-only context
          # and returns a list of action tuples.
          if new_state.on_token do
            ctx = build_context(new_state)
            actions = new_state.on_token.(token, ctx)
            apply_actions(actions, token, new_state, tokens)
          else
            {:continue, new_state, [token | tokens]}
          end

        {:error, msg} ->
          {:error, msg}

        :no_match ->
          {:error,
           "Line #{state.line}, column #{state.column}: Unexpected character: #{inspect(ch)}"}
      end
    end
  end

  # -- Build a LexerContext from the current state -----------------------------

  defp build_context(%State{} = state) do
    %LexerContext{
      active_group: List.first(state.group_stack),
      group_stack_depth: length(state.group_stack),
      source: state.source,
      pos_after_token: state.pos,
      available_groups: Map.keys(state.group_patterns)
    }
  end

  # -- Apply callback actions -------------------------------------------------
  #
  # The callback returns a list of action tuples. We process them in order:
  #
  # 1. Collect all actions into categories (suppress, emits, group ops, skip toggle)
  # 2. If :suppress is present, don't add the current token
  # 3. Add any emitted tokens after the current token
  # 4. Apply group stack operations (push/pop)
  # 5. Apply skip toggle if present
  #
  # This matches the Python behavior where actions take effect after the
  # callback returns — they don't interrupt the current match.

  defp apply_actions(actions, token, state, tokens) do
    # Parse actions into structured categories
    suppressed = :suppress in actions

    emitted_tokens =
      actions
      |> Enum.filter(fn
        {:emit, _tok} -> true
        _ -> false
      end)
      |> Enum.map(fn {:emit, tok} -> tok end)

    # Build the new token list:
    # - If not suppressed, prepend the current token
    # - Then prepend emitted tokens (in reverse order since tokens list is reversed)
    new_tokens =
      if suppressed do
        tokens
      else
        [token | tokens]
      end

    # Emitted tokens go after the current token. Since our list is built
    # in reverse, we prepend them in reverse order so they appear in the
    # correct order when the final list is reversed.
    new_tokens =
      Enum.reduce(Enum.reverse(emitted_tokens), new_tokens, fn tok, acc ->
        [tok | acc]
      end)

    # Apply group stack operations
    new_state =
      Enum.reduce(actions, state, fn
        {:push_group, group_name}, acc ->
          if Map.has_key?(acc.group_patterns, group_name) do
            %{acc | group_stack: [group_name | acc.group_stack]}
          else
            # Unknown group — raise an error (matches Python behavior).
            # In practice, this shouldn't happen if the callback validates
            # group names against ctx.available_groups.
            raise ArgumentError,
                  "Unknown pattern group: #{inspect(group_name)}. " <>
                    "Available groups: #{inspect(Enum.sort(Map.keys(acc.group_patterns)))}"
          end

        :pop_group, acc ->
          # Pop the current group. If only "default" remains, this is a
          # no-op — the default group is the floor and cannot be popped.
          case acc.group_stack do
            [_only_one] -> acc
            [_top | rest] -> %{acc | group_stack: rest}
          end

        {:set_skip_enabled, enabled}, acc ->
          %{acc | skip_enabled: enabled}

        # :suppress and {:emit, _} were already handled above
        _, acc ->
          acc
      end)

    {:continue, new_state, new_tokens}
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
  #
  # `try_match_token_in_group/3` tries each compiled pattern from a specific
  # group in priority order (first match wins). This replaces the old
  # `try_match_token/2` which only used the default patterns.

  defp try_match_token_in_group(remaining, state, group_patterns) do
    Enum.reduce_while(group_patterns, :no_match, fn {token_name, pattern}, _acc ->
      case Regex.run(pattern, remaining, capture: :first) do
        [match] ->
          start_line = state.line
          start_column = state.column

          # Resolve token type (keyword detection, alias, etc.)
          case resolve_token_type(token_name, match, state) do
            {:ok, token_type} ->
              # Handle STRING tokens: strip quotes and process escapes.
              # When escape_mode is "none", we strip quotes but leave escape
              # sequences as raw text — the semantic layer handles them.
              # This is used by grammars like TOML where different string types
              # have different escape semantics.
              effective_name = Map.get(state.alias_map, token_name, token_name)

              value =
                if effective_name == "STRING" or token_name == "STRING" do
                  stripped = String.slice(match, 1..-2//1)

                  if state.escape_mode == "none" do
                    stripped
                  else
                    process_escapes(stripped)
                  end
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
