defmodule CodingAdventures.LispParser do
  @moduledoc """
  Lisp parser backed by the shared grammar-driven parser engine.

  The parser reads `lisp.grammar` from `code/grammars/`, tokenizes source with
  `CodingAdventures.LispLexer`, and delegates AST construction to
  `CodingAdventures.Parser.GrammarParser`.
  """

  alias CodingAdventures.GrammarTools.ParserGrammar
  alias CodingAdventures.LispLexer
  alias CodingAdventures.Parser.{ASTNode, GrammarParser}

  @grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "..", "grammars"])
                |> Path.expand()

  @doc """
  Parse Lisp source code and return `{:ok, ast}` or `{:error, message}`.
  """
  @spec parse(String.t()) :: {:ok, ASTNode.t()} | {:error, String.t()}
  def parse(source) when is_binary(source) do
    case LispLexer.tokenize(source) do
      {:ok, tokens} -> GrammarParser.parse(tokens, create_parser())
      {:error, message} -> {:error, message}
    end
  end

  @doc """
  Parse and return the cached Lisp `ParserGrammar`.
  """
  @spec create_parser() :: ParserGrammar.t()
  def create_parser do
    case :persistent_term.get({__MODULE__, :grammar}, nil) do
      nil ->
        grammar_path = Path.join(@grammars_dir, "lisp.grammar")
        {:ok, grammar} = ParserGrammar.parse(File.read!(grammar_path))
        :persistent_term.put({__MODULE__, :grammar}, grammar)
        grammar

      grammar ->
        grammar
    end
  end
end
