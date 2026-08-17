defmodule CodingAdventures.LispParser do
  @moduledoc """
  Lisp parser backed by a pre-compiled, in-source grammar.

  `CodingAdventures.LispParser.Grammar` embeds the parsed `lisp.grammar`
  grammar as native Elixir data (generated via `grammar-tools compile-grammar`
  from `code/grammars/lisp/lisp.grammar`). The parser tokenizes source with
  `CodingAdventures.LispLexer` and delegates AST construction to
  `CodingAdventures.Parser.GrammarParser`.
  """

  alias CodingAdventures.GrammarTools.ParserGrammar
  alias CodingAdventures.LispLexer
  alias CodingAdventures.LispParser.Grammar
  alias CodingAdventures.Parser.{ASTNode, GrammarParser}

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
  Return the cached Lisp `ParserGrammar`.
  """
  @spec create_parser() :: ParserGrammar.t()
  def create_parser do
    case :persistent_term.get({__MODULE__, :grammar}, nil) do
      nil ->
        grammar = Grammar.parser_grammar()
        :persistent_term.put({__MODULE__, :grammar}, grammar)
        grammar

      grammar ->
        grammar
    end
  end
end
