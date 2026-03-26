defmodule CodingAdventures.ExcelParser do
  @moduledoc "Excel formula parser built on the shared grammar-driven parser."

  alias CodingAdventures.ExcelLexer
  alias CodingAdventures.GrammarTools.ParserGrammar
  alias CodingAdventures.Parser.{ASTNode, GrammarParser}

  @grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "..", "grammars"]) |> Path.expand()

  def normalize_excel_reference_tokens(tokens) do
    Enum.with_index(tokens)
    |> Enum.map(fn {token, index} ->
      if token.type not in ["NAME", "NUMBER"] do
        token
      else
        previous = previous_significant_token(tokens, index)
        following = next_significant_token(tokens, index)
        adjacent_to_colon =
          (previous && previous.type == "COLON") || (following && following.type == "COLON")

        cond do
          token.type == "NAME" and adjacent_to_colon ->
            %{token | type: "COLUMN_REF"}

          token.type == "NUMBER" and adjacent_to_colon ->
            %{token | type: "ROW_REF"}

          true ->
            token
        end
      end
    end)
  end

  defp previous_significant_token(tokens, index) do
    if index <= 0 do
      nil
    else
      0..(index - 1)
      |> Enum.reverse()
      |> Enum.find_value(fn i ->
        token = Enum.at(tokens, i)
        if token && token.type != "SPACE", do: token, else: nil
      end)
    end
  end

  defp next_significant_token(tokens, index) do
    if index >= length(tokens) - 1 do
      nil
    else
      (index + 1)..(length(tokens) - 1)
      |> Enum.find_value(fn i ->
        token = Enum.at(tokens, i)
        if token && token.type != "SPACE", do: token, else: nil
      end)
    end
  end

  @spec parse(String.t()) :: {:ok, ASTNode.t()} | {:error, String.t()}
  def parse(source) do
    grammar = get_grammar()

    case ExcelLexer.tokenize(source) do
      {:ok, tokens} ->
        GrammarParser.parse(tokens, grammar, pre_parse_hooks: [&normalize_excel_reference_tokens/1])

      {:error, msg} ->
        {:error, msg}
    end
  end

  def create_parser do
    grammar_path = Path.join(@grammars_dir, "excel.grammar")
    {:ok, grammar} = ParserGrammar.parse(File.read!(grammar_path))
    grammar
  end

  defp get_grammar do
    case :persistent_term.get({__MODULE__, :grammar}, nil) do
      nil ->
        grammar = create_parser()
        :persistent_term.put({__MODULE__, :grammar}, grammar)
        grammar

      grammar ->
        grammar
    end
  end
end
