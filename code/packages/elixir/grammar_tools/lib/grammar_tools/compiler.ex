defmodule CodingAdventures.GrammarTools.Compiler do
  @moduledoc """
  Compile `TokenGrammar` and `ParserGrammar` structs into Elixir source code.

  The grammar-tools library parses `.tokens` and `.grammar` files into
  in-memory data structures. This module adds the *compile* step: given a
  parsed grammar object, generate Elixir source code that instantiates the
  grammar as native Elixir structs — eliminating all file I/O and parse
  overhead at runtime.

  ## Why compile grammars?

  The default workflow reads `.tokens` and `.grammar` files at startup. This
  has three costs that compilation eliminates:

  1. **File I/O at startup** — every process must find and open the files.
     Packages walk up the directory tree to find `code/grammars/`, coupling
     them to the repo layout.

  2. **Parse overhead at startup** — the grammar is re-parsed every run.

  3. **Deployment coupling** — `.tokens` and `.grammar` files must ship
     alongside the compiled binary.

  ## Generated output shape (json.tokens → json_tokens.ex)

      # AUTO-GENERATED FILE — DO NOT EDIT
      # Source: json.tokens

      defmodule Generated.JsonTokens do
        @moduledoc false
        alias CodingAdventures.GrammarTools.TokenGrammar

        def token_grammar do
          %TokenGrammar{
            definitions: [
              %{name: "STRING", pattern: "\\"[^\\"]*\\"", is_regex: true, line_number: 1, alias: nil},
            ],
            keywords: [],
            version: 0,
            case_insensitive: false,
          }
        end
      end

  ## Design notes

  - All helpers are `defp` (private) — only the two public functions form the API.
  - Elixir's `inspect/1` is used for string literals, giving correct quoting
    and escape sequences without any manual escaping logic.
  - Grammar elements are rendered as tagged tuples matching the types defined in
    `CodingAdventures.GrammarTools.ParserGrammar`:
    `{:rule_reference, name, is_token}`, `{:literal, value}`, etc.
  """

  alias CodingAdventures.GrammarTools.{TokenGrammar, ParserGrammar}

  # ===========================================================================
  # Public API
  # ===========================================================================

  @doc """
  Generate Elixir source code embedding a `TokenGrammar` as native data.

  ## Parameters

  - `grammar`     — a `TokenGrammar` struct to compile.
  - `source_file` — the original `.tokens` filename for the header comment.
    Pass `""` to omit the `Source:` line.

  Returns a `String` of valid Elixir source code.  Write it to a `.ex` file.
  """
  @spec compile_token_grammar(TokenGrammar.t(), String.t()) :: String.t()
  def compile_token_grammar(%TokenGrammar{} = grammar, source_file \\ "") do
    # Strip newlines so a crafted filename cannot break out of the comment line
    # and inject arbitrary code into the generated file.
    source_file = String.replace(source_file, ~r/[\r\n]/, "_")
    source_line = if source_file == "", do: "", else: "# Source: #{source_file}\n"
    defs_src = token_def_list_src(grammar.definitions, "      ")
    skip_src = token_def_list_src(grammar.skip_definitions, "      ")
    err_src = token_def_list_src(grammar.error_definitions, "      ")
    groups_src = groups_src(grammar.groups, "      ")

    """
    # AUTO-GENERATED FILE \u2014 DO NOT EDIT
    #{source_line}# Regenerate with: grammar-tools compile-tokens #{source_file}
    #
    # This file embeds a TokenGrammar as native Elixir data structures.
    # Call token_grammar/0 instead of reading and parsing the .tokens file.

    alias CodingAdventures.GrammarTools.TokenGrammar

    def token_grammar do
      %TokenGrammar{
        definitions: #{defs_src},
        keywords: #{inspect(grammar.keywords)},
        mode: #{inspect(grammar.mode)},
        escape_mode: #{inspect(grammar.escape_mode)},
        skip_definitions: #{skip_src},
        reserved_keywords: #{inspect(grammar.reserved_keywords)},
        error_definitions: #{err_src},
        groups: #{groups_src},
        layout_keywords: #{inspect(grammar.layout_keywords)},
        case_sensitive: #{grammar.case_sensitive},
        version: #{grammar.version},
        case_insensitive: #{grammar.case_insensitive},
      }
    end
    """
  end

  @doc """
  Generate Elixir source code embedding a `ParserGrammar` as native data.

  ## Parameters

  - `grammar`     — a `ParserGrammar` struct to compile.
  - `source_file` — the original `.grammar` filename for the header comment.

  Returns a `String` of valid Elixir source code.
  """
  @spec compile_parser_grammar(ParserGrammar.t(), String.t()) :: String.t()
  def compile_parser_grammar(%ParserGrammar{} = grammar, source_file \\ "") do
    # Strip newlines so a crafted filename cannot break out of the comment line.
    source_file = String.replace(source_file, ~r/[\r\n]/, "_")
    source_line = if source_file == "", do: "", else: "# Source: #{source_file}\n"

    rules_src =
      if grammar.rules == [] do
        "[]"
      else
        inner = grammar.rules |> Enum.map(&grammar_rule_src(&1, "      ")) |> Enum.join(",\n")
        "[\n#{inner},\n    ]"
      end

    """
    # AUTO-GENERATED FILE \u2014 DO NOT EDIT
    #{source_line}# Regenerate with: grammar-tools compile-grammar #{source_file}
    #
    # This file embeds a ParserGrammar as native Elixir data structures.
    # Call parser_grammar/0 instead of reading and parsing the .grammar file.

    alias CodingAdventures.GrammarTools.ParserGrammar

    def parser_grammar do
      %ParserGrammar{
        rules: #{rules_src},
        version: #{grammar.version},
      }
    end
    """
  end

  # ===========================================================================
  # Token grammar helpers
  # ===========================================================================

  # Render one token definition map as an Elixir map literal.
  defp token_def_src(defn, indent) do
    i = indent <> "  "

    "#{indent}%{\n" <>
      "#{i}name: #{inspect(defn.name)},\n" <>
      "#{i}pattern: #{inspect(defn.pattern)},\n" <>
      "#{i}is_regex: #{defn.is_regex},\n" <>
      "#{i}line_number: #{defn.line_number},\n" <>
      "#{i}alias: #{inspect(defn.alias)},\n" <>
      "#{indent}}"
  end

  # Render a list of token definitions as an Elixir list literal.
  defp token_def_list_src(defs, _indent) when defs == [], do: "[]"

  defp token_def_list_src(defs, indent) do
    inner = indent <> "  "
    items = defs |> Enum.map(&token_def_src(&1, inner)) |> Enum.join(",\n")
    "[\n#{items},\n#{indent}]"
  end

  # Render the groups map as an Elixir map literal.
  defp groups_src(groups, _indent) when groups == %{}, do: "%{}"

  defp groups_src(groups, indent) do
    inner = indent <> "  "
    inner2 = inner <> "  "

    entries =
      groups
      |> Enum.map(fn {name, group} ->
        defs_lit = token_def_list_src(group.definitions, inner2 <> "  ")

        "#{inner}#{inspect(name)} => %{\n" <>
          "#{inner2}name: #{inspect(group.name)},\n" <>
          "#{inner2}definitions: #{defs_lit},\n" <>
          "#{inner}}"
      end)
      |> Enum.join(",\n")

    "%{\n#{entries},\n#{indent}}"
  end

  # ===========================================================================
  # Parser grammar helpers
  # ===========================================================================

  # Render one grammar_rule map as an Elixir map literal.
  defp grammar_rule_src(rule, indent) do
    i = indent <> "  "
    body_src = element_src(rule.body, i)

    "#{indent}%{\n" <>
      "#{i}name: #{inspect(rule.name)},\n" <>
      "#{i}body: #{body_src},\n" <>
      "#{i}line_number: #{rule.line_number},\n" <>
      "#{indent}}"
  end

  # Recursively render a grammar_element (tagged tuple) as an Elixir expression.
  #
  # Elixir uses tagged tuples for grammar elements, e.g.:
  #   {:rule_reference, "value", false}
  #   {:alternation, [...]}
  defp element_src({:rule_reference, name, is_token}, _indent) do
    "{:rule_reference, #{inspect(name)}, #{is_token}}"
  end

  defp element_src({:literal, value}, _indent) do
    "{:literal, #{inspect(value)}}"
  end

  defp element_src({:sequence, elements}, indent) do
    i = indent <> "  "
    items = elements |> Enum.map(&"#{i}#{element_src(&1, i)}") |> Enum.join(",\n")
    "{:sequence, [\n#{items},\n#{indent}]}"
  end

  defp element_src({:alternation, choices}, indent) do
    i = indent <> "  "
    items = choices |> Enum.map(&"#{i}#{element_src(&1, i)}") |> Enum.join(",\n")
    "{:alternation, [\n#{items},\n#{indent}]}"
  end

  defp element_src({:repetition, element}, indent) do
    i = indent <> "  "
    child = element_src(element, i)
    "{:repetition, #{child}}"
  end

  defp element_src({:optional, element}, indent) do
    i = indent <> "  "
    child = element_src(element, i)
    "{:optional, #{child}}"
  end

  defp element_src({:group, element}, indent) do
    i = indent <> "  "
    child = element_src(element, i)
    "{:group, #{child}}"
  end
end
