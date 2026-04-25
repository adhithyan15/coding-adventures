defmodule CodingAdventures.CssParser do
  @moduledoc """
  CSS parser backed by the shared grammar-driven parser engine.

  The parser reads `css.grammar` from `code/grammars/`, tokenizes source with
  `CodingAdventures.CssLexer`, and delegates AST construction to
  `CodingAdventures.Parser.GrammarParser`.
  """

  alias CodingAdventures.CssLexer
  alias CodingAdventures.GrammarTools.ParserGrammar
  alias CodingAdventures.Parser.{ASTNode, GrammarParser}

  @grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "grammars"])
                |> Path.expand()

  @doc """
  Parse CSS source code and return `{:ok, ast}` or `{:error, message}`.
  """
  @spec parse(String.t()) :: {:ok, ASTNode.t()} | {:error, String.t()}
  def parse(source) when is_binary(source) do
    case CssLexer.tokenize(source) do
      {:ok, tokens} -> GrammarParser.parse(tokens, create_parser())
      {:error, message} -> {:error, message}
    end
  end

  @doc """
  Parse and return the cached CSS `ParserGrammar`.
  """
  @spec create_parser() :: ParserGrammar.t()
  def create_parser do
    case :persistent_term.get({__MODULE__, :grammar}, nil) do
      nil ->
        grammar_path = Path.join(@grammars_dir, "css.grammar")
        {:ok, grammar} = ParserGrammar.parse(File.read!(grammar_path))
        :persistent_term.put({__MODULE__, :grammar}, grammar)
        grammar

      grammar ->
        grammar
    end
  end
end
