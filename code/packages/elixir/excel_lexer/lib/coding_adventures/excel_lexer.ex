defmodule CodingAdventures.ExcelLexer do
  @moduledoc "Excel formula lexer built on the shared grammar-driven lexer."

  alias CodingAdventures.GrammarTools.TokenGrammar
  alias CodingAdventures.Lexer.GrammarLexer

  @grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "..", "grammars"]) |> Path.expand()

  defp next_non_space_char_from_source(source, token) do
    start_index = source_index_after_token(source, token)

    source
    |> String.slice(start_index..-1//1)
    |> to_charlist()
    |> Enum.find(fn char -> char != ?\s end)
    |> case do
      nil -> ""
      char -> <<char::utf8>>
    end
  end

  defp source_index_after_token(source, token) do
    prefix =
      source
      |> String.split("\n")
      |> Enum.take(token.line - 1)
      |> Enum.map(&("#{&1}\n"))
      |> Enum.join()

    String.length(prefix) + token.column - 1 + String.length(token.value)
  end

  def tokenize(source) do
    grammar = get_grammar()

    GrammarLexer.tokenize(
      source,
      grammar,
      on_token: fn token, _ctx ->
        if token.type != "NAME" do
          []
        else
          case next_non_space_char_from_source(source, token) do
            "(" ->
              [
                :suppress,
                {:emit,
                 %CodingAdventures.Lexer.Token{
                   type: "FUNCTION_NAME",
                   value: token.value,
                   line: token.line,
                   column: token.column
                 }}
              ]

            "[" ->
              [
                :suppress,
                {:emit,
                 %CodingAdventures.Lexer.Token{
                   type: "TABLE_NAME",
                   value: token.value,
                   line: token.line,
                   column: token.column
                 }}
              ]

            _ ->
              []
          end
        end
      end
    )
  end

  def create_lexer do
    tokens_path = Path.join(@grammars_dir, "excel.tokens")
    {:ok, grammar} = TokenGrammar.parse(File.read!(tokens_path))

    definitions =
      Enum.map(grammar.definitions, fn definition ->
        pattern =
          cond do
            definition.name in ["FUNCTION_NAME", "TABLE_NAME", "COLUMN_REF", "ROW_REF"] ->
              "a^"

            definition.is_regex and not String.starts_with?(definition.pattern, "^") ->
              "^(?:#{definition.pattern})"

            true ->
              definition.pattern
          end

        %{definition | pattern: pattern}
      end)

    skip_definitions =
      Enum.map(grammar.skip_definitions, fn definition ->
        if definition.is_regex and not String.starts_with?(definition.pattern, "^") do
          %{definition | pattern: "^(?:#{definition.pattern})"}
        else
          definition
        end
      end)

    %{grammar | definitions: definitions, skip_definitions: skip_definitions}
  end

  defp get_grammar do
    case :persistent_term.get({__MODULE__, :grammar}, nil) do
      nil ->
        grammar = create_lexer()
        :persistent_term.put({__MODULE__, :grammar}, grammar)
        grammar

      grammar ->
        grammar
    end
  end
end
