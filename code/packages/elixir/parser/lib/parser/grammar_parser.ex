defmodule CodingAdventures.Parser.GrammarParser do
  @moduledoc """
  Grammar-Driven Parser — Parsing from .grammar Files
  =====================================================

  This parser reads grammar rules from a `ParserGrammar` (parsed from a
  `.grammar` file) and interprets them at runtime. The same Elixir code
  can parse JSON, Python, Ruby, or any language — just swap the grammar.

  ## How It Works

  The parser receives two inputs:

  1. A list of `Token` structs from the lexer.
  2. A `ParserGrammar` with EBNF rules (from `grammar_tools`).

  It walks the grammar rule tree, trying to match each element against
  the token stream. Each EBNF element type has a natural interpretation:

  - **{:rule_reference, name, true}** — Match a token of that type.
  - **{:rule_reference, name, false}** — Recursively parse that grammar rule.
  - **{:sequence, elements}** — Match all elements in order.
  - **{:alternation, choices}** — Try each choice; first match wins.
  - **{:repetition, element}** — Match zero or more times.
  - **{:optional, element}** — Match zero or one time.
  - **{:literal, value}** — Match a token by exact text value.
  - **{:group, element}** — Parenthesized sub-expression.

  ## Backtracking and Packrat Memoization

  When an alternation's first choice fails, the parser restores position
  and tries the next choice (backtracking). To avoid exponential behavior,
  we cache results keyed by `{rule_name, position}` (packrat memoization).

  ## State Threading

  Since Elixir is immutable, parser state is threaded through every function
  as a `%State{}` struct. Each function returns `{result, updated_state}`.
  """

  alias CodingAdventures.Parser.ASTNode
  alias CodingAdventures.Lexer.Token
  alias CodingAdventures.GrammarTools.ParserGrammar

  # -- Parser state -----------------------------------------------------------

  defmodule State do
    @moduledoc false
    defstruct [
      :tokens,
      :rules,
      :newlines_significant,
      pos: 0,
      memo: %{},
      furthest_pos: 0,
      furthest_expected: [],
      trace: false
    ]
  end

  # -- Public API -------------------------------------------------------------

  @doc """
  Parse a list of tokens using a `ParserGrammar`.

  The first rule in the grammar is the entry point. Returns
  `{:ok, ast_node}` on success, `{:error, message}` on failure.

  ## Options

  - `:pre_parse_hooks` — a list of functions `([Token.t()] -> [Token.t()])`
    that transform the token list before parsing begins. Hooks are applied
    in order (left to right) via `Enum.reduce/3`. This is useful for
    token-level preprocessing — for example, filtering out whitespace
    tokens, injecting synthetic tokens, or reordering tokens.

  - `:post_parse_hooks` — a list of functions `(ASTNode.t() -> ASTNode.t())`
    that transform the AST after parsing succeeds. Hooks are applied in
    order (left to right) via `Enum.reduce/3`. This is useful for AST
    post-processing — for example, constant folding, dead code elimination,
    or desugaring.

  - `trace: false` (default) — when set to `true`, emits a line to stderr
    for each rule attempt. This helps diagnose parse failures by showing
    which rules were tried at which token positions.

    Trace format:
    ```
    [TRACE] rule 'name' at token 0 (TYPE "value") → match
    [TRACE] rule 'name' at token 1 (TYPE "value") → fail
    ```

  ## Examples

      # Without hooks (original behavior):
      {:ok, ast} = GrammarParser.parse(tokens, grammar)

      # With pre-parse hook (filter out NEWLINE tokens):
      {:ok, ast} = GrammarParser.parse(tokens, grammar,
        pre_parse_hooks: [fn toks -> Enum.reject(toks, & &1.type == "NEWLINE") end])

      # With post-parse hook (annotate AST):
      {:ok, ast} = GrammarParser.parse(tokens, grammar,
        post_parse_hooks: [fn ast -> %{ast | metadata: :annotated} end])
  """
  @spec parse([Token.t()], ParserGrammar.t(), keyword()) ::
          {:ok, ASTNode.t()} | {:error, String.t()}
  def parse(tokens, grammar, opts \\ [])

  def parse(tokens, %ParserGrammar{rules: rules} = _grammar, opts) when is_list(tokens) do
    pre_parse_hooks = Keyword.get(opts, :pre_parse_hooks, [])
    post_parse_hooks = Keyword.get(opts, :post_parse_hooks, [])

    if rules == [] do
      {:error, "Grammar has no rules"}
    else
      # Stage 1: Pre-parse hooks transform the token list.
      # Each hook is a function ([Token.t()] -> [Token.t()]) that receives
      # the token list and returns a transformed version. Hooks run left to
      # right, so the output of hook N becomes the input of hook N+1.
      tokens = Enum.reduce(pre_parse_hooks, tokens, fn hook, toks -> hook.(toks) end)

      # Build lookup map: rule_name -> rule
      rule_map = Map.new(rules, fn r -> {r.name, r} end)

      # Detect newline significance
      newlines_sig = Enum.any?(rules, fn r -> element_references_newline?(r.body) end)

      trace = Keyword.get(opts, :trace, false)

      state = %State{
        tokens: :array.from_list(tokens),
        rules: rule_map,
        newlines_significant: newlines_sig,
        pos: 0,
        trace: trace
      }

      token_count = length(tokens)
      entry_rule = hd(rules).name

      # Stage 2: Parse the token list.
      case parse_rule(entry_rule, state) do
        {:ok, node, state} ->
          # Skip trailing newlines
          state = skip_trailing_newlines(state, token_count)

          # Verify all tokens consumed (except EOF)
          current = current_token(state, token_count)

          if state.pos < token_count and current.type != "EOF" do
            {:error, format_error(state, current)}
          else
            # Stage 3: Post-parse hooks transform the AST.
            # Each hook is a function (ASTNode.t() -> ASTNode.t()) that
            # receives the AST and returns a transformed version. Only
            # applied on success — errors pass through unchanged.
            node = Enum.reduce(post_parse_hooks, node, fn hook, ast -> hook.(ast) end)
            {:ok, node}
          end

        {:error, msg, _state} ->
          {:error, msg}
      end
    end
  end

  # -- Newline significance detection ----------------------------------------

  defp element_references_newline?({:rule_reference, "NEWLINE", true}), do: true
  defp element_references_newline?({:rule_reference, _, _}), do: false
  defp element_references_newline?({:literal, _}), do: false

  defp element_references_newline?({:sequence, elements}),
    do: Enum.any?(elements, &element_references_newline?/1)

  defp element_references_newline?({:alternation, choices}),
    do: Enum.any?(choices, &element_references_newline?/1)

  defp element_references_newline?({tag, element})
       when tag in [:repetition, :optional, :group],
       do: element_references_newline?(element)

  # -- Helpers ----------------------------------------------------------------

  defp current_token(%State{tokens: tokens, pos: pos}, token_count) do
    if pos < token_count do
      :array.get(pos, tokens)
    else
      :array.get(token_count - 1, tokens)
    end
  end

  defp skip_trailing_newlines(state, token_count) do
    if state.pos < token_count do
      token = :array.get(state.pos, state.tokens)

      if token.type == "NEWLINE" do
        skip_trailing_newlines(%{state | pos: state.pos + 1}, token_count)
      else
        state
      end
    else
      state
    end
  end

  defp record_failure(state, expected) do
    cond do
      state.pos > state.furthest_pos ->
        %{state | furthest_pos: state.pos, furthest_expected: [expected]}

      state.pos == state.furthest_pos and expected not in state.furthest_expected ->
        %{state | furthest_expected: state.furthest_expected ++ [expected]}

      true ->
        state
    end
  end

  defp format_error(state, current) do
    if state.furthest_expected != [] and state.furthest_pos > state.pos do
      expected_str = state.furthest_expected |> Enum.take(5) |> Enum.join(" or ")
      token_count = :array.size(state.tokens)
      furthest_tok = current_token(%{state | pos: state.furthest_pos}, token_count)

      "Parse error at #{furthest_tok.line}:#{furthest_tok.column}: " <>
        "Expected #{expected_str}, got #{inspect(furthest_tok.value)}"
    else
      "Parse error at #{current.line}:#{current.column}: " <>
        "Unexpected token: #{inspect(current.value)}"
    end
  end

  # -- Rule parsing with packrat memoization ----------------------------------

  defp parse_rule(rule_name, state) do
    case Map.fetch(state.rules, rule_name) do
      :error ->
        {:error, "Undefined rule: #{rule_name}", state}

      {:ok, rule} ->
        memo_key = {rule_name, state.pos}

        case Map.fetch(state.memo, memo_key) do
          {:ok, {nil, end_pos}} ->
            current = current_token(state, :array.size(state.tokens))
            emit_trace(state, rule_name, state.pos, current, :fail)

            {:error,
             "Parse error at #{current.line}:#{current.column}: " <>
               "Expected #{rule_name}, got #{inspect(current.value)}",
             %{state | pos: end_pos}}

          {:ok, {children, end_pos}} ->
            current = current_token(state, :array.size(state.tokens))
            emit_trace(state, rule_name, state.pos, current, :match)
            node = %ASTNode{rule_name: rule_name, children: children}
            {:ok, node, %{state | pos: end_pos}}

          :error ->
            start_pos = state.pos
            current_at_start = current_token(state, :array.size(state.tokens))

            case match_element(rule.body, state) do
              {:ok, children, state} ->
                emit_trace(state, rule_name, start_pos, current_at_start, :match)
                state = %{state | memo: Map.put(state.memo, memo_key, {children, state.pos})}
                node = %ASTNode{rule_name: rule_name, children: children}
                {:ok, node, state}

              {:fail, state} ->
                emit_trace(state, rule_name, start_pos, current_at_start, :fail)
                state = %{state | pos: start_pos}
                state = record_failure(state, rule_name)
                state = %{state | memo: Map.put(state.memo, memo_key, {nil, state.pos})}
                current = current_token(state, :array.size(state.tokens))

                {:error,
                 "Parse error at #{current.line}:#{current.column}: " <>
                   "Expected #{rule_name}, got #{inspect(current.value)}",
                 state}
            end
        end
    end
  end

  # Emit a trace line to stderr when tracing is enabled.
  # Format: [TRACE] rule 'name' at token N (TYPE "value") → match|fail
  defp emit_trace(%State{trace: true}, rule_name, pos, token, result) do
    outcome = if result == :match, do: "match", else: "fail"
    IO.write(:stderr, "[TRACE] rule '#{rule_name}' at token #{pos} (#{token.type} #{inspect(token.value)}) \u2192 #{outcome}\n")
  end

  defp emit_trace(%State{trace: false}, _rule_name, _pos, _token, _result), do: :ok

  # -- Element matching (the core grammar interpreter) ------------------------
  #
  # All match_element/2 clauses are grouped together to avoid Elixir warnings.
  # Helper functions (match_sequence, match_alternation, match_repetition) are
  # defined after the match_element clauses.

  # Sequence: A B C — all must match in order
  defp match_element({:sequence, elements}, state) do
    save_pos = state.pos
    match_sequence(elements, state, [], save_pos)
  end

  # Alternation: A | B | C — try each, first match wins
  defp match_element({:alternation, choices}, state) do
    save_pos = state.pos
    match_alternation(choices, state, save_pos)
  end

  # Repetition: { A } — zero or more
  defp match_element({:repetition, inner}, state) do
    match_repetition(inner, state, [])
  end

  # Optional: [ A ] — zero or one
  defp match_element({:optional, inner}, state) do
    case match_element(inner, state) do
      {:ok, result, state} -> {:ok, result, state}
      {:fail, _state_after} -> {:ok, [], state}
    end
  end

  # Group: ( A ) — just delegation
  defp match_element({:group, inner}, state) do
    match_element(inner, state)
  end

  # Rule reference (token): match current token by type
  defp match_element({:rule_reference, name, true}, state) do
    token_count = :array.size(state.tokens)

    # Skip insignificant newlines
    state =
      if not state.newlines_significant and name != "NEWLINE" do
        skip_newlines(state, token_count)
      else
        state
      end

    token = current_token(state, token_count)

    if token.type == name do
      {:ok, [token], %{state | pos: state.pos + 1}}
    else
      state = record_failure(state, name)
      {:fail, state}
    end
  end

  # Rule reference (rule): recursively parse another grammar rule
  defp match_element({:rule_reference, name, false}, state) do
    save_pos = state.pos

    case parse_rule(name, state) do
      {:ok, node, state} -> {:ok, [node], state}
      {:error, _msg, state} -> {:fail, %{state | pos: save_pos}}
    end
  end

  # Literal: match token by exact text value
  defp match_element({:literal, value}, state) do
    token_count = :array.size(state.tokens)

    # Skip insignificant newlines
    state =
      if not state.newlines_significant do
        skip_newlines(state, token_count)
      else
        state
      end

    token = current_token(state, token_count)

    if token.value == value do
      {:ok, [token], %{state | pos: state.pos + 1}}
    else
      state = record_failure(state, ~s("#{value}"))
      {:fail, state}
    end
  end

  # -- Element matching helpers ------------------------------------------------

  defp match_sequence([], state, children, _save_pos) do
    {:ok, Enum.reverse(children), state}
  end

  defp match_sequence([elem | rest], state, children, save_pos) do
    case match_element(elem, state) do
      {:ok, result, state} ->
        match_sequence(rest, state, Enum.reverse(result) ++ children, save_pos)

      {:fail, state} ->
        {:fail, %{state | pos: save_pos}}
    end
  end

  defp match_alternation([], state, save_pos) do
    {:fail, %{state | pos: save_pos}}
  end

  defp match_alternation([choice | rest], state, save_pos) do
    state = %{state | pos: save_pos}

    case match_element(choice, state) do
      {:ok, result, state} -> {:ok, result, state}
      {:fail, state} -> match_alternation(rest, state, save_pos)
    end
  end

  defp match_repetition(inner, state, children) do
    save_pos = state.pos

    case match_element(inner, state) do
      {:ok, result, state} ->
        match_repetition(inner, state, Enum.reverse(result) ++ children)

      {:fail, state} ->
        {:ok, Enum.reverse(children), %{state | pos: save_pos}}
    end
  end

  defp skip_newlines(state, token_count) do
    if state.pos < token_count do
      token = :array.get(state.pos, state.tokens)

      if token.type == "NEWLINE" do
        skip_newlines(%{state | pos: state.pos + 1}, token_count)
      else
        state
      end
    else
      state
    end
  end
end
