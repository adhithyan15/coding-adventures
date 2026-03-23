defmodule CodingAdventures.GrammarTools.ParserGrammar do
  @moduledoc """
  Parses and validates `.grammar` files (EBNF notation).

  A `.grammar` file describes the syntactic structure of a language using
  Extended Backus-Naur Form (EBNF). Each rule maps a name to a body
  expression built from sequences, alternations, repetitions, optionals,
  groups, token references, rule references, and literals.

  ## Grammar Element Types (tagged tuples)

      {:rule_reference, name, is_token}   — reference to a rule or token
      {:literal, value}                   — literal string match
      {:sequence, elements}               — A B C (all must match in order)
      {:alternation, choices}             — A | B | C (first match wins)
      {:repetition, element}              — { A } (zero or more)
      {:optional, element}                — [ A ] (zero or one)
      {:group, element}                   — ( A ) (grouping)
  """

  defstruct rules: []

  @type grammar_element ::
          {:rule_reference, String.t(), boolean()}
          | {:literal, String.t()}
          | {:sequence, [grammar_element()]}
          | {:alternation, [grammar_element()]}
          | {:repetition, grammar_element()}
          | {:optional, grammar_element()}
          | {:group, grammar_element()}

  @type grammar_rule :: %{
          name: String.t(),
          body: grammar_element(),
          line_number: pos_integer()
        }

  @type t :: %__MODULE__{rules: [grammar_rule()]}

  @doc """
  Parse the text of a `.grammar` file into a `ParserGrammar` struct.

  Returns `{:ok, grammar}` on success, `{:error, message}` on failure.
  """
  @spec parse(String.t()) :: {:ok, t()} | {:error, String.t()}
  def parse(source) do
    case tokenize(source) do
      {:ok, tokens} ->
        case parse_rules(tokens, []) do
          {:ok, rules, _rest} -> {:ok, %__MODULE__{rules: rules}}
          {:error, msg} -> {:error, msg}
        end

      {:error, msg} ->
        {:error, msg}
    end
  end

  @doc "Return all defined rule names."
  @spec rule_names(t()) :: MapSet.t(String.t())
  def rule_names(%__MODULE__{rules: rules}) do
    rules |> Enum.map(& &1.name) |> MapSet.new()
  end

  @doc "Return all UPPERCASE token references in the grammar."
  @spec token_references(t()) :: MapSet.t(String.t())
  def token_references(%__MODULE__{rules: rules}) do
    rules
    |> Enum.flat_map(fn rule -> collect_refs(rule.body, :token) end)
    |> MapSet.new()
  end

  @doc "Return all lowercase rule references in the grammar."
  @spec rule_references(t()) :: MapSet.t(String.t())
  def rule_references(%__MODULE__{rules: rules}) do
    rules
    |> Enum.flat_map(fn rule -> collect_refs(rule.body, :rule) end)
    |> MapSet.new()
  end

  @doc """
  Validate a parsed `ParserGrammar` for common problems.

  Validation checks:

  - **Duplicate rule names** — two rules with the same name.
  - **Non-lowercase rule names** — by convention, rule names are lowercase.
  - **Undefined rule references** — a lowercase name used in a body but never
    defined as a rule.
  - **Undefined token references** — an UPPERCASE name used but not in the
    provided `token_names` set (only checked when `token_names` is given).
  - **Unreachable rules** — a rule that is defined but never referenced by
    any other rule. The first rule (start symbol) is exempt.

  Synthetic tokens (`NEWLINE`, `INDENT`, `DEDENT`, `EOF`) are always
  considered valid token references regardless of the token grammar.

  Returns a list of issue strings. An empty list means no problems found.
  """
  @spec validate_parser_grammar(t(), MapSet.t(String.t()) | nil) :: [String.t()]
  def validate_parser_grammar(%__MODULE__{} = grammar, token_names \\ nil) do
    issues = []
    defined = rule_names(grammar)
    referenced_rules = rule_references(grammar)
    referenced_tokens = token_references(grammar)

    # --- Duplicate rule names ---
    {issues, _seen} =
      Enum.reduce(grammar.rules, {issues, %{}}, fn rule, {acc_issues, seen} ->
        if Map.has_key?(seen, rule.name) do
          first_line = seen[rule.name]

          {acc_issues ++
             [
               "Line #{rule.line_number}: Duplicate rule name '#{rule.name}' " <>
                 "(first defined on line #{first_line})"
             ], seen}
        else
          {acc_issues, Map.put(seen, rule.name, rule.line_number)}
        end
      end)

    # --- Non-lowercase rule names ---
    issues =
      Enum.reduce(grammar.rules, issues, fn rule, acc ->
        if rule.name != String.downcase(rule.name) do
          acc ++
            ["Line #{rule.line_number}: Rule name '#{rule.name}' should be lowercase"]
        else
          acc
        end
      end)

    # --- Undefined rule references ---
    issues =
      referenced_rules
      |> MapSet.to_list()
      |> Enum.sort()
      |> Enum.reduce(issues, fn ref, acc ->
        if not MapSet.member?(defined, ref) do
          acc ++ ["Undefined rule reference: '#{ref}'"]
        else
          acc
        end
      end)

    # --- Undefined token references ---
    # Synthetic tokens produced by the lexer without explicit .tokens
    # definitions: NEWLINE (bare newlines), INDENT/DEDENT (indentation mode),
    # EOF (always emitted at end of input).
    synthetic_tokens = MapSet.new(["NEWLINE", "INDENT", "DEDENT", "EOF"])

    issues =
      if token_names != nil do
        referenced_tokens
        |> MapSet.to_list()
        |> Enum.sort()
        |> Enum.reduce(issues, fn ref, acc ->
          if not MapSet.member?(token_names, ref) and not MapSet.member?(synthetic_tokens, ref) do
            acc ++ ["Undefined token reference: '#{ref}'"]
          else
            acc
          end
        end)
      else
        issues
      end

    # --- Unreachable rules ---
    # The first rule is the start symbol; all other rules that are never
    # referenced by any other rule are unreachable dead code.
    issues =
      if grammar.rules != [] do
        start_rule = hd(grammar.rules).name

        Enum.reduce(grammar.rules, issues, fn rule, acc ->
          if rule.name != start_rule and not MapSet.member?(referenced_rules, rule.name) do
            acc ++
              [
                "Line #{rule.line_number}: Rule '#{rule.name}' is defined but " <>
                  "never referenced (unreachable)"
              ]
          else
            acc
          end
        end)
      else
        issues
      end

    issues
  end

  # -- Collect references from grammar elements -----------------------------

  defp collect_refs({:rule_reference, name, true}, :token), do: [name]
  defp collect_refs({:rule_reference, name, false}, :rule), do: [name]
  defp collect_refs({:rule_reference, _, _}, _), do: []
  defp collect_refs({:literal, _}, _), do: []

  defp collect_refs({:sequence, elements}, kind),
    do: Enum.flat_map(elements, &collect_refs(&1, kind))

  defp collect_refs({:alternation, choices}, kind),
    do: Enum.flat_map(choices, &collect_refs(&1, kind))

  defp collect_refs({tag, element}, kind) when tag in [:repetition, :optional, :group],
    do: collect_refs(element, kind)

  # -- Tokenizer for .grammar files -----------------------------------------
  # Breaks the grammar source into tokens: IDENT, STRING, EQUALS, SEMI, PIPE,
  # LBRACE, RBRACE, LBRACKET, RBRACKET, LPAREN, RPAREN, EOF.

  defp tokenize(source) do
    lines = String.split(source, "\n")

    result =
      lines
      |> Enum.with_index(1)
      |> Enum.reduce_while([], fn {raw_line, line_num}, acc ->
        line = String.trim_trailing(raw_line)
        stripped = String.trim(line)

        if stripped == "" or String.starts_with?(stripped, "#") do
          {:cont, acc}
        else
          case tokenize_line(line, 0, line_num, acc) do
            {:ok, new_acc} -> {:cont, new_acc}
            {:error, msg} -> {:halt, {:error, msg}}
          end
        end
      end)

    case result do
      {:error, msg} -> {:error, msg}
      tokens -> {:ok, Enum.reverse([{:eof, "", 0} | tokens])}
    end
  end

  defp tokenize_line(line, pos, _line_num, acc) when pos >= byte_size(line), do: {:ok, acc}

  defp tokenize_line(line, pos, line_num, acc) do
    ch = String.at(line, pos)

    cond do
      ch in [" ", "\t"] ->
        tokenize_line(line, pos + 1, line_num, acc)

      ch == "#" ->
        {:ok, acc}

      ch == "=" ->
        tokenize_line(line, pos + 1, line_num, [{:equals, "=", line_num} | acc])

      ch == ";" ->
        tokenize_line(line, pos + 1, line_num, [{:semi, ";", line_num} | acc])

      ch == "|" ->
        tokenize_line(line, pos + 1, line_num, [{:pipe, "|", line_num} | acc])

      ch == "{" ->
        tokenize_line(line, pos + 1, line_num, [{:lbrace, "{", line_num} | acc])

      ch == "}" ->
        tokenize_line(line, pos + 1, line_num, [{:rbrace, "}", line_num} | acc])

      ch == "[" ->
        tokenize_line(line, pos + 1, line_num, [{:lbracket, "[", line_num} | acc])

      ch == "]" ->
        tokenize_line(line, pos + 1, line_num, [{:rbracket, "]", line_num} | acc])

      ch == "(" ->
        tokenize_line(line, pos + 1, line_num, [{:lparen, "(", line_num} | acc])

      ch == ")" ->
        tokenize_line(line, pos + 1, line_num, [{:rparen, ")", line_num} | acc])

      ch == "\"" ->
        start = pos + 1
        case read_string(line, start, line_num) do
          {:ok, closing_pos} ->
            value = String.slice(line, start, closing_pos - start)
            tokenize_line(line, closing_pos + 1, line_num, [{:string, value, line_num} | acc])

          {:error, msg} ->
            {:error, msg}
        end

      is_alpha_or_underscore(ch) ->
        {ident, end_pos} = read_ident(line, pos)
        tokenize_line(line, end_pos, line_num, [{:ident, ident, line_num} | acc])

      true ->
        {:error, "Line #{line_num}: Unexpected character: #{inspect(ch)}"}
    end
  end

  defp is_alpha_or_underscore(ch) do
    (ch >= "a" and ch <= "z") or (ch >= "A" and ch <= "Z") or ch == "_"
  end

  defp is_alnum_or_underscore(ch) do
    is_alpha_or_underscore(ch) or (ch >= "0" and ch <= "9")
  end

  defp read_string(line, pos, line_num) do
    cond do
      pos >= byte_size(line) ->
        {:error, "Line #{line_num}: Unterminated string literal"}

      String.at(line, pos) == "\\" and pos + 1 < byte_size(line) ->
        read_string(line, pos + 2, line_num)

      String.at(line, pos) == "\"" ->
        {:ok, pos}

      true ->
        read_string(line, pos + 1, line_num)
    end
  end

  defp read_ident(line, start_pos) do
    end_pos = find_ident_end(line, start_pos)
    {String.slice(line, start_pos, end_pos - start_pos), end_pos}
  end

  defp find_ident_end(line, pos) do
    if pos < byte_size(line) and is_alnum_or_underscore(String.at(line, pos)) do
      find_ident_end(line, pos + 1)
    else
      pos
    end
  end

  # -- Recursive descent parser for EBNF ------------------------------------
  # Grammar of grammars:
  #   rules     = { rule } EOF
  #   rule      = IDENT "=" body ";"
  #   body      = sequence { "|" sequence }
  #   sequence  = element { element }
  #   element   = IDENT | STRING | "{" body "}" | "[" body "]" | "(" body ")"

  defp parse_rules([{:eof, _, _} | _] = tokens, rules) do
    {:ok, Enum.reverse(rules), tokens}
  end

  defp parse_rules(tokens, rules) do
    case parse_rule(tokens) do
      {:ok, rule, rest} -> parse_rules(rest, [rule | rules])
      {:error, msg} -> {:error, msg}
    end
  end

  defp parse_rule([{:ident, name, line_num} | rest]) do
    case rest do
      [{:equals, _, _} | rest2] ->
        case parse_body(rest2) do
          {:ok, body, [{:semi, _, _} | rest3]} ->
            {:ok, %{name: name, body: body, line_number: line_num}, rest3}

          {:ok, _body, [{kind, val, ln} | _]} ->
            {:error, "Line #{ln}: Expected ';', got #{kind} (#{inspect(val)})"}

          {:error, msg} ->
            {:error, msg}
        end

      [{kind, val, ln} | _] ->
        {:error, "Line #{ln}: Expected '=', got #{kind} (#{inspect(val)})"}

      [] ->
        {:error, "Unexpected end of input after rule name '#{name}'"}
    end
  end

  defp parse_rule([{kind, val, ln} | _]) do
    {:error, "Line #{ln}: Expected rule name (identifier), got #{kind} (#{inspect(val)})"}
  end

  defp parse_body(tokens) do
    case parse_sequence(tokens) do
      {:ok, first, rest} ->
        collect_alternation(rest, [first])

      {:error, msg} ->
        {:error, msg}
    end
  end

  defp collect_alternation([{:pipe, _, _} | rest], choices) do
    case parse_sequence(rest) do
      {:ok, next, rest2} -> collect_alternation(rest2, [next | choices])
      {:error, msg} -> {:error, msg}
    end
  end

  defp collect_alternation(tokens, [single]) do
    {:ok, single, tokens}
  end

  defp collect_alternation(tokens, choices) do
    {:ok, {:alternation, Enum.reverse(choices)}, tokens}
  end

  defp parse_sequence(tokens) do
    case collect_elements(tokens, []) do
      {:ok, [], _rest} ->
        [{_, _, ln} | _] = tokens
        {:error, "Line #{ln}: Expected at least one element in sequence"}

      {:ok, [single], rest} ->
        {:ok, single, rest}

      {:ok, elements, rest} ->
        {:ok, {:sequence, Enum.reverse(elements)}, rest}
    end
  end

  defp collect_elements([{kind, _, _} | _] = tokens, elements)
       when kind in [:pipe, :semi, :rbrace, :rbracket, :rparen, :eof] do
    {:ok, elements, tokens}
  end

  defp collect_elements(tokens, elements) do
    case parse_element(tokens) do
      {:ok, element, rest} -> collect_elements(rest, [element | elements])
      {:error, msg} -> {:error, msg}
    end
  end

  defp parse_element([{:ident, name, _} | rest]) do
    is_token = name == String.upcase(name) and String.match?(String.at(name, 0), ~r/[A-Z]/)
    {:ok, {:rule_reference, name, is_token}, rest}
  end

  defp parse_element([{:string, value, _} | rest]) do
    {:ok, {:literal, value}, rest}
  end

  defp parse_element([{:lbrace, _, _} | rest]) do
    case parse_body(rest) do
      {:ok, body, [{:rbrace, _, _} | rest2]} ->
        {:ok, {:repetition, body}, rest2}

      {:ok, _, [{kind, val, ln} | _]} ->
        {:error, "Line #{ln}: Expected '}', got #{kind} (#{inspect(val)})"}

      {:error, msg} ->
        {:error, msg}
    end
  end

  defp parse_element([{:lbracket, _, _} | rest]) do
    case parse_body(rest) do
      {:ok, body, [{:rbracket, _, _} | rest2]} ->
        {:ok, {:optional, body}, rest2}

      {:ok, _, [{kind, val, ln} | _]} ->
        {:error, "Line #{ln}: Expected ']', got #{kind} (#{inspect(val)})"}

      {:error, msg} ->
        {:error, msg}
    end
  end

  defp parse_element([{:lparen, _, _} | rest]) do
    case parse_body(rest) do
      {:ok, body, [{:rparen, _, _} | rest2]} ->
        {:ok, {:group, body}, rest2}

      {:ok, _, [{kind, val, ln} | _]} ->
        {:error, "Line #{ln}: Expected ')', got #{kind} (#{inspect(val)})"}

      {:error, msg} ->
        {:error, msg}
    end
  end

  defp parse_element([{kind, val, ln} | _]) do
    {:error, "Line #{ln}: Unexpected token: #{kind} (#{inspect(val)})"}
  end
end
