defmodule CodingAdventures.CssLexer do
  @moduledoc """
  CSS lexer backed by the shared grammar-driven lexer engine.

  This package embeds a pre-compiled `css.tokens` grammar (see
  `CodingAdventures.CssLexer.Grammar`) and delegates tokenization to
  `CodingAdventures.Lexer.GrammarLexer`.
  """

  alias CodingAdventures.CssLexer.Grammar
  alias CodingAdventures.GrammarTools.TokenGrammar
  alias CodingAdventures.Lexer.GrammarLexer

  @spec tokenize(String.t()) :: {:ok, [CodingAdventures.Lexer.Token.t()]} | {:error, String.t()}
  def tokenize(source) do
    grammar = get_grammar()
    GrammarLexer.tokenize(source, grammar)
  end

  @spec create_lexer() :: TokenGrammar.t()
  def create_lexer do
    Grammar.token_grammar()
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
