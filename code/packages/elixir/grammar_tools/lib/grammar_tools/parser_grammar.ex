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
