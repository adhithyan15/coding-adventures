defmodule CodingAdventures.LispLexer do
  @moduledoc """
  Lisp lexer backed by a pre-compiled, in-source grammar.

  `CodingAdventures.LispLexer.Grammar` embeds the parsed `lisp.tokens`
  grammar as native Elixir data (generated via `grammar-tools compile-tokens`
  from `code/grammars/lisp/lisp.tokens`). The lexer caches that grammar in
  `:persistent_term` and delegates tokenization to
  `CodingAdventures.Lexer.GrammarLexer`.
  """

  alias CodingAdventures.GrammarTools.TokenGrammar
  alias CodingAdventures.Lexer.GrammarLexer
  alias CodingAdventures.LispLexer.Grammar

  @doc """
  Tokenize Lisp source code and return `{:ok, tokens}` or `{:error, message}`.
  """
  @spec tokenize(String.t()) ::
          {:ok, [CodingAdventures.Lexer.Token.t()]} | {:error, String.t()}
  def tokenize(source) when is_binary(source) do
    GrammarLexer.tokenize(source, create_lexer())
  end

  @doc """
  Return the cached Lisp `TokenGrammar`.
  """
  @spec create_lexer() :: TokenGrammar.t()
  def create_lexer do
    case :persistent_term.get({__MODULE__, :grammar}, nil) do
      nil ->
        grammar = Grammar.token_grammar()
        :persistent_term.put({__MODULE__, :grammar}, grammar)
        grammar

      grammar ->
        grammar
    end
  end
end
