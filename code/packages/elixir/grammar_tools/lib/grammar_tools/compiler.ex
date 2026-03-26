defmodule CodingAdventures.GrammarTools.Compiler do
  @moduledoc """
  Compiles TokenGrammar and ParserGrammar into native Elixir modules.
  """

  alias CodingAdventures.GrammarTools.{TokenGrammar, ParserGrammar}

  @doc """
  Compiles a `TokenGrammar` to an Elixir module definition string.
  """
  def compile_tokens_to_elixir(%TokenGrammar{} = grammar, export_name) do
    """
    # AUTO-GENERATED FILE - DO NOT EDIT
    defmodule #{export_name} do
      alias CodingAdventures.GrammarTools.TokenGrammar

      def grammar do
        %TokenGrammar{
          version: #{grammar.version},
          case_insensitive: #{grammar.case_insensitive},
          case_sensitive: #{grammar.case_sensitive},
          mode: #{inspect(grammar.mode)},
          escape_mode: #{inspect(grammar.escape_mode)},
          keywords: #{inspect(grammar.keywords)},
          reserved_keywords: #{inspect(grammar.reserved_keywords)},
          definitions: [
    #{compile_token_defs(grammar.definitions, "        ")}
          ],
          skip_definitions: [
    #{compile_token_defs(grammar.skip_definitions, "        ")}
          ],
          error_definitions: [
    #{compile_token_defs(grammar.error_definitions, "        ")}
          ],
          groups: %{
    #{compile_token_groups(grammar.groups, "        ")}
          }
        }
      end
    end
    """
  end

  defp compile_token_defs(defs, indent) do
    Enum.map(defs, fn d ->
      "#{indent}%{name: #{inspect(d.name)}, pattern: #{inspect(d.pattern)}, is_regex: #{d.is_regex}, line_number: #{d.line_number}, alias: #{inspect(d.alias)}}"
    end)
    |> Enum.join(",\n")
  end

  defp compile_token_groups(groups, indent) do
    groups
    |> Enum.sort_by(fn {name, _} -> name end)
    |> Enum.map(fn {name, group} ->
      """
      #{indent}#{inspect(name)} => %{
      #{indent}  name: #{inspect(group.name)},
      #{indent}  definitions: [
      #{compile_token_defs(group.definitions, "#{indent}    ")}
      #{indent}  ]
      #{indent}}
      """ |> String.trim_trailing()
    end)
    |> Enum.join(",\n")
  end

  @doc """
  Compiles a `ParserGrammar` to an Elixir module definition string.
  """
  def compile_parser_to_elixir(%ParserGrammar{} = grammar, export_name) do
    """
    # AUTO-GENERATED FILE - DO NOT EDIT
    defmodule #{export_name} do
      alias CodingAdventures.GrammarTools.ParserGrammar

      def grammar do
        %ParserGrammar{
          version: #{grammar.version},
          rules: [
    #{compile_parser_rules(grammar.rules, "        ")}
          ]
        }
      end
    end
    """
  end

  defp compile_parser_rules(rules, indent) do
    Enum.map(rules, fn r ->
      "#{indent}%{name: #{inspect(r.name)}, line_number: #{r.line_number}, body: #{compile_parser_el(r.body)}}"
    end)
    |> Enum.join(",\n")
  end

  defp compile_parser_el({:rule_reference, name, is_token}) do
    "{:rule_reference, #{inspect(name)}, #{is_token}}"
  end
  defp compile_parser_el({:literal, value}) do
    "{:literal, #{inspect(value)}}"
  end
  defp compile_parser_el({:sequence, elements}) do
    elems = Enum.map(elements, &compile_parser_el/1) |> Enum.join(", ")
    "{:sequence, [#{elems}]}"
  end
  defp compile_parser_el({:alternation, choices}) do
    elems = Enum.map(choices, &compile_parser_el/1) |> Enum.join(", ")
    "{:alternation, [#{elems}]}"
  end
  defp compile_parser_el({:repetition, element}) do
    "{:repetition, #{compile_parser_el(element)}}"
  end
  defp compile_parser_el({:optional, element}) do
    "{:optional, #{compile_parser_el(element)}}"
  end
  defp compile_parser_el({:group, element}) do
    "{:group, #{compile_parser_el(element)}}"
  end
end
