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
      :available_groups,
      :previous_token,
      :bracket_depths,
      :current_token_line
    ]

    @type t :: %__MODULE__{
            active_group: String.t(),
            group_stack_depth: pos_integer(),
            source: String.t(),
            pos_after_token: non_neg_integer(),
            available_groups: [String.t()],
            previous_token: Token.t() | nil,
            bracket_depths: %{paren: non_neg_integer(), bracket: non_neg_integer(), brace: non_neg_integer()},
            current_token_line: pos_integer()
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

    @doc """
    Return the most recently emitted token, or `nil` at the start of input.

    "Emitted" means the token actually made it into the output list --
    suppressed tokens are not counted. This provides **lookbehind**
    capability for context-sensitive decisions.

    For example, in JavaScript `/` is a regex literal after `=`, `(`
    or `,` but a division operator after `)`, `]`, identifiers, or
    numbers. The callback can check `LexerContext.previous_token(ctx)`
    to decide which interpretation to use.
    """
    @spec previous_token(t()) :: Token.t() | nil
    def previous_token(%__MODULE__{previous_token: prev}), do: prev

    @doc """
    Return the current nesting depth for a specific bracket type,
    or the total depth across all types if `kind` is `:all`.

    Depth starts at 0 and increments on each opener (`(`, `[`, `{`),
    decrements on each closer (`)`, `]`, `}`). The count never goes
    below 0 -- unmatched closers are clamped.

    This is essential for template literal interpolation in languages
    like JavaScript, Kotlin, and Ruby, where `}` at brace-depth 0
    closes the interpolation rather than being part of a nested
    expression.

    ## Examples

        iex> LexerContext.bracket_depth(ctx, :paren)
        2
        iex> LexerContext.bracket_depth(ctx, :all)
        5
    """
    @spec bracket_depth(t(), :paren | :bracket | :brace | :all) :: non_neg_integer()
    def bracket_depth(%__MODULE__{bracket_depths: depths}, :all) do
      depths.paren + depths.bracket + depths.brace
    end

    def bracket_depth(%__MODULE__{bracket_depths: depths}, kind)
        when kind in [:paren, :bracket, :brace] do
      Map.fetch!(depths, kind)
    end

    @doc """
    Return true if a newline appeared between the previous token
    and the current token (i.e., they are on different lines).

    This is used by languages with automatic semicolon insertion
    (JavaScript, Go) to detect line breaks that trigger implicit
    statement termination.

    Returns false if there is no previous token (start of input).
    """
    @spec preceded_by_newline(t()) :: boolean()
    def preceded_by_newline(%__MODULE__{previous_token: nil}), do: false

    def preceded_by_newline(%__MODULE__{previous_token: prev, current_token_line: current_line}) do
      prev.line < current_line
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
      # -- Context keywords ---
      # MapSet of context-sensitive keywords (emitted as NAME with flag).
      :context_keyword_set,
      :layout_keyword_set,
      # In Elixir, bare atoms (:field) must come before keyword pairs
      # (field: default) in defstruct lists. That is why group_patterns
      # appears above and case_insensitive / group_stack / etc. appear below.
      # -- Case-insensitive keyword matching ---
      # When true, keywords are stored as uppercase and matched
      # case-insensitively. The emitted KEYWORD value is also uppercase.
      case_insensitive: false,
      # The group stack. Bottom is always "default". Top is the active group.
      group_stack: ["default"],
      # -- Callback ---
      # Optional function (Token, LexerContext) -> [action]
      on_token: nil,
      # Whether skip patterns are active. Callbacks can toggle this.
      skip_enabled: true,
      # -- Extension: Token lookbehind ---
      # The most recently emitted token, for lookbehind in callbacks.
      last_emitted_token: nil,
      # -- Extension: Bracket depth tracking ---
      # Per-type bracket nesting depth counters: %{paren: 0, bracket: 0, brace: 0}
      bracket_depths: %{paren: 0, bracket: 0, brace: 0},
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

  - `:pre_tokenize_hooks` — a list of functions `(String.t() -> String.t())`
    that transform the source text before tokenization begins. Hooks are
    applied in order (left to right) via `Enum.reduce/3`. This is useful
    for source-level preprocessing — for example, stripping a BOM,
    normalizing line endings, or expanding macros.

  - `:post_tokenize_hooks` — a list of functions `([Token.t()] -> [Token.t()])`
    that transform the token list after tokenization succeeds. Hooks are
    applied in order (left to right) via `Enum.reduce/3`. This is useful
    for post-processing — for example, filtering out comment tokens,
    inserting synthetic tokens, or rewriting token types.

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

      # With pre-tokenize hook (normalize line endings):
      {:ok, tokens} = GrammarLexer.tokenize(source, grammar,
        pre_tokenize_hooks: [&String.replace(&1, "\\r\\n", "\\n")])

      # With post-tokenize hook (filter out NEWLINE tokens):
      {:ok, tokens} = GrammarLexer.tokenize(source, grammar,
        post_tokenize_hooks: [fn toks -> Enum.reject(toks, & &1.type == "NEWLINE") end])
  """
  @spec tokenize(String.t(), TokenGrammar.t(), keyword()) ::
          {:ok, [Token.t()]} | {:error, String.t()}
  def tokenize(source, grammar, opts \\ [])

  def tokenize(source, %TokenGrammar{} = grammar, opts) do
    on_token = Keyword.get(opts, :on_token, nil)
    pre_tokenize_hooks = Keyword.get(opts, :pre_tokenize_hooks, [])
    post_tokenize_hooks = Keyword.get(opts, :post_tokenize_hooks, [])

    # Stage 1: Pre-tokenize hooks transform the source text.
    # Each hook is a function (String.t() -> String.t()) that receives the
    # source and returns a transformed version. Hooks run left to right,
    # so the output of hook N becomes the input of hook N+1.
    source = Enum.reduce(pre_tokenize_hooks, source, fn hook, src -> hook.(src) end)

    # Case-insensitive mode: lowercase the entire source before matching.
    # This mirrors the Python GrammarLexer behavior — keyword promotion and
    # pattern matching both operate on the lowercased text, which is the
    # correct behavior for case-insensitive languages like VHDL or SQL.
    #
    # Exception: when `case_insensitive: true` is ALSO set, the lexer uses
    # the smarter Elixir-native approach (upcase lookup on keywords only,
    # identifiers preserve their original case). In that mode we do NOT
    # lowercase the source — the keyword normalisation in Stage 3 handles
    # case folding only for KEYWORD tokens. This lets `case_sensitive: false`
    # in shared grammar files coexist with Elixir's `case_insensitive: true`
    # without accidentally casefolding identifier values.
    effective_source =
      if grammar.case_sensitive or grammar.case_insensitive,
        do: source,
        else: String.downcase(source)

    state = init_state(effective_source, grammar, on_token)

    tokenizer =
      case grammar.mode do
        "layout" -> &tokenize_layout/2
        _ -> &tokenize_standard/2
      end

    # Stage 2: Tokenize the source.
    case tokenizer.(state, []) do
      {:ok, tokens} ->
        # Stage 3: Post-tokenize hooks transform the token list.
        # Each hook is a function ([Token.t()] -> [Token.t()]) that receives
        # the token list and returns a transformed version. Only applied on
        # success — errors pass through unchanged.
        tokens = Enum.reduce(post_tokenize_hooks, tokens, fn hook, toks -> hook.(toks) end)
        {:ok, tokens}

      error ->
        error
    end
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

    # Build the keyword set. When case_insensitive is true, all keywords
    # are stored as uppercase so that comparisons can be done with
    # String.upcase(value) — a classic "normalize on insert" strategy.
    keyword_set =
      if grammar.case_insensitive do
        grammar.keywords
        |> Enum.map(&String.upcase/1)
        |> MapSet.new()
      else
        MapSet.new(grammar.keywords)
      end

    # Build context keyword set for O(1) lookup.
    context_keyword_set =
      MapSet.new(Map.get(grammar, :context_keywords, []) || [])

    layout_keyword_set =
      MapSet.new(Map.get(grammar, :layout_keywords, []) || [])

    %State{
      source: source,
      patterns: patterns,
      skip_patterns: skip_patterns,
      keyword_set: keyword_set,
      reserved_set: MapSet.new(grammar.reserved_keywords),
      alias_map: merged_alias_map,
      has_skip_patterns: length(grammar.skip_definitions) > 0,
      escape_mode: grammar.escape_mode,
      case_insensitive: grammar.case_insensitive,
      group_patterns: group_patterns,
      context_keyword_set: context_keyword_set,
      layout_keyword_set: layout_keyword_set,
      group_stack: ["default"],
      on_token: on_token,
      skip_enabled: true,
      last_emitted_token: nil,
      bracket_depths: %{paren: 0, bracket: 0, brace: 0}
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

  defp tokenize_layout(state, tokens) do
    case tokenize_standard(state, tokens) do
      {:ok, toks} -> {:ok, apply_layout(toks, state.layout_keyword_set)}
      other -> other
    end
  end

  defp apply_layout(tokens, layout_keyword_set) do
    {result, _layout_stack, _pending_layouts, _suppress_depth} =
      Enum.with_index(tokens)
      |> Enum.reduce({[], [], 0, 0}, fn {token, index}, {acc, stack, pending, suppress_depth} ->
        cond do
          token.type == "NEWLINE" ->
            next_token = next_layout_token(tokens, index + 1)

            {acc, stack} =
              if suppress_depth == 0 and next_token do
                dedented =
                  Enum.reduce_while(stack, {acc, stack}, fn column, {inner_acc, inner_stack} ->
                    if next_token.column < column do
                      {:cont,
                       {[virtual_layout_token("VIRTUAL_RBRACE", "}", next_token) | inner_acc],
                        tl(inner_stack)}}
                    else
                      {:halt, {inner_acc, inner_stack}}
                    end
                  end)

                {inner_acc, inner_stack} = dedented

                inner_acc =
                  if inner_stack != [] and next_token.type != "EOF" and next_token.value != "}" and next_token.column == hd(inner_stack) do
                    [virtual_layout_token("VIRTUAL_SEMICOLON", ";", next_token) | inner_acc]
                  else
                    inner_acc
                  end

                {inner_acc, inner_stack}
              else
                {acc, stack}
              end

            {[token | acc], stack, pending, suppress_depth}

          token.type == "EOF" ->
            closing = Enum.map(stack, fn _ -> virtual_layout_token("VIRTUAL_RBRACE", "}", token) end)
            {Enum.reverse(closing) ++ [token | acc], [], pending, suppress_depth}

          pending > 0 and token.value == "{" ->
            new_suppress = update_layout_suppress_depth(suppress_depth, token)
            {[token | acc], stack, pending - 1, new_suppress}

          pending > 0 ->
            injected = Enum.map(1..pending, fn _ -> virtual_layout_token("VIRTUAL_LBRACE", "{", token) end)
            new_stack = Enum.map(1..pending, fn _ -> token.column end) ++ stack
            new_suppress = update_layout_suppress_depth(suppress_depth, token)
            {[token | Enum.reverse(injected) ++ acc], new_stack, 0, new_suppress}

          true ->
            new_suppress = update_layout_suppress_depth(suppress_depth, token)
            new_pending = if layout_keyword?(layout_keyword_set, token), do: pending + 1, else: pending
            {[token | acc], stack, new_pending, new_suppress}
        end
      end)

    result |> Enum.reverse()
  end

  defp next_layout_token(tokens, start_index) do
    tokens
    |> Enum.drop(start_index)
    |> Enum.find(fn token -> token.type != "NEWLINE" end)
  end

  defp virtual_layout_token(type, value, anchor) do
    %Token{type: type, value: value, line: anchor.line, column: anchor.column}
  end

  defp update_layout_suppress_depth(depth, %Token{type: type, value: value}) do
    cond do
      String.starts_with?(type, "VIRTUAL_") -> depth
      value in ["(", "[", "{"] -> depth + 1
      value in [")", "]", "}"] and depth > 0 -> depth - 1
      true -> depth
    end
  end

  defp layout_keyword?(layout_keyword_set, %Token{value: value}) do
    MapSet.member?(layout_keyword_set, value) or MapSet.member?(layout_keyword_set, String.downcase(value))
  end

  defp try_newline_or_token(remaining, state, tokens) do
    ch = binary_part(remaining, 0, 1)

    if ch == "\n" do
      token = %Token{type: "NEWLINE", value: "\\n", line: state.line, column: state.column}
      new_state = %{advance(state) | last_emitted_token: token}
      {:continue, new_state, [token | tokens]}
    else
      # Use the active group's patterns (top of the group stack).
      # When no groups are defined, this is always "default", preserving
      # backward compatibility with grammars that don't use groups.
      active_group = List.first(state.group_stack)
      group_pats = Map.get(state.group_patterns, active_group, state.patterns)

      case try_match_token_in_group(remaining, state, group_pats) do
        {:matched, token, new_state} ->
          # Update bracket depth tracking.
          new_state = update_bracket_depth(new_state, token.value)

          # If a callback is registered, invoke it and process the
          # returned actions. The callback receives a read-only context
          # and returns a list of action tuples.
          if new_state.on_token do
            ctx = build_context(new_state, token.line)
            actions = new_state.on_token.(token, ctx)
            apply_actions(actions, token, new_state, tokens)
          else
            new_state = %{new_state | last_emitted_token: token}
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

  defp build_context(%State{} = state, nil) do
    build_context(state, state.line)
  end

  defp build_context(%State{} = state, token_line) when is_integer(token_line) do
    %LexerContext{
      active_group: List.first(state.group_stack),
      group_stack_depth: length(state.group_stack),
      source: state.source,
      pos_after_token: state.pos,
      available_groups: Map.keys(state.group_patterns),
      previous_token: state.last_emitted_token,
      bracket_depths: state.bracket_depths,
      current_token_line: token_line
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

    # Track last emitted token through suppression and emission.
    # If not suppressed, the current token is last; then any emitted
    # tokens become last in order.
    last_emitted =
      cond do
        emitted_tokens != [] -> List.last(emitted_tokens)
        not suppressed -> token
        true -> state.last_emitted_token
      end

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

    new_state = %{new_state | last_emitted_token: last_emitted}

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

              # When case_insensitive is true, KEYWORD values are normalized to
              # uppercase. This means "select", "SELECT", and "Select" all
              # produce KEYWORD tokens with value "SELECT", making the emitted
              # token stream uniform regardless of how the user typed the keyword.
              value =
                if token_type == "KEYWORD" and state.case_insensitive do
                  String.upcase(value)
                else
                  value
                end

              # Check if this NAME token is a context keyword — a word that
              # is sometimes a keyword and sometimes an identifier depending
              # on syntactic position. Context keywords are emitted as NAME
              # with the TOKEN_CONTEXT_KEYWORD flag, leaving the final
              # decision to the language-specific parser or callback.
              flags =
                if token_type == "NAME" and
                     MapSet.size(state.context_keyword_set) > 0 and
                     MapSet.member?(state.context_keyword_set, value) do
                  Token.context_keyword()
                else
                  nil
                end

              token = %Token{
                type: token_type,
                value: value,
                line: start_line,
                column: start_column,
                flags: flags
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

    # When case_insensitive is true, compare against the keyword/reserved sets
    # using the uppercased value. The keyword set was built with uppercase keys
    # (see init_state), so this is a consistent "normalize on both sides" check.
    lookup_value = if state.case_insensitive, do: String.upcase(value), else: value

    cond do
      effective_name == "NAME" and MapSet.member?(state.reserved_set, lookup_value) ->
        {:error,
         "Line #{state.line}, column #{state.column}: " <>
           "Reserved keyword '#{value}' cannot be used as an identifier"}

      effective_name == "NAME" and MapSet.member?(state.keyword_set, lookup_value) ->
        {:ok, "KEYWORD"}

      true ->
        {:ok, effective_name}
    end
  end

  # -- Bracket depth tracking --------------------------------------------------
  #
  # Update bracket depth counters based on a token's value.
  # Only single-character values are checked — multi-character tokens
  # cannot be brackets.

  defp update_bracket_depth(state, "("),
    do: %{state | bracket_depths: %{state.bracket_depths | paren: state.bracket_depths.paren + 1}}

  defp update_bracket_depth(state, ")") do
    new = max(state.bracket_depths.paren - 1, 0)
    %{state | bracket_depths: %{state.bracket_depths | paren: new}}
  end

  defp update_bracket_depth(state, "["),
    do: %{state | bracket_depths: %{state.bracket_depths | bracket: state.bracket_depths.bracket + 1}}

  defp update_bracket_depth(state, "]") do
    new = max(state.bracket_depths.bracket - 1, 0)
    %{state | bracket_depths: %{state.bracket_depths | bracket: new}}
  end

  defp update_bracket_depth(state, "{"),
    do: %{state | bracket_depths: %{state.bracket_depths | brace: state.bracket_depths.brace + 1}}

  defp update_bracket_depth(state, "}") do
    new = max(state.bracket_depths.brace - 1, 0)
    %{state | bracket_depths: %{state.bracket_depths | brace: new}}
  end

  defp update_bracket_depth(state, _value), do: state

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
