defmodule CodingAdventures.RubyParser do
  @moduledoc """
  Ruby parser backed by the shared grammar-driven parser engine.

  The parser imports a pre-compiled `CodingAdventures.RubyParser.Grammar`
  module (generated from `ruby.grammar` via `grammar-tools compile-grammar`),
  tokenizes source with `CodingAdventures.RubyLexer`, and delegates AST
  construction to `CodingAdventures.Parser.GrammarParser`.
  """

  alias CodingAdventures.GrammarTools.ParserGrammar
  alias CodingAdventures.Parser.{ASTNode, GrammarParser}
  alias CodingAdventures.RubyLexer
  alias CodingAdventures.RubyParser.Grammar

  @doc """
  Parse Ruby source code and return `{:ok, ast}` or `{:error, message}`.
  """
  @spec parse(String.t()) :: {:ok, ASTNode.t()} | {:error, String.t()}
  def parse(source) when is_binary(source) do
    case RubyLexer.tokenize(source) do
      {:ok, tokens} -> GrammarParser.parse(tokens, create_parser())
      {:error, message} -> {:error, message}
    end
  end

  @doc """
  Parse and return the cached Ruby `ParserGrammar`.
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
