defmodule CodingAdventures.LispLexer do
  @moduledoc """
  Lisp lexer backed by the shared grammar-driven lexer engine.

  The lexer reads `lisp.tokens` from `code/grammars/`, parses the file into a
  `TokenGrammar`, caches it in `:persistent_term`, and delegates tokenization
  to `CodingAdventures.Lexer.GrammarLexer`.
  """

  alias CodingAdventures.GrammarTools.TokenGrammar
  alias CodingAdventures.Lexer.GrammarLexer

  @grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "..", "grammars"])
                |> Path.expand()

  @doc """
  Tokenize Lisp source code and return `{:ok, tokens}` or `{:error, message}`.
  """
  @spec tokenize(String.t()) ::
          {:ok, [CodingAdventures.Lexer.Token.t()]} | {:error, String.t()}
  def tokenize(source) when is_binary(source) do
    GrammarLexer.tokenize(source, create_lexer())
  end

  @doc """
  Parse and return the cached Lisp `TokenGrammar`.
  """
  @spec create_lexer() :: TokenGrammar.t()
  def create_lexer do
    case :persistent_term.get({__MODULE__, :grammar}, nil) do
      nil ->
        tokens_path = Path.join(@grammars_dir, "lisp.tokens")
        {:ok, grammar} = TokenGrammar.parse(File.read!(tokens_path))
        :persistent_term.put({__MODULE__, :grammar}, grammar)
        grammar

      grammar ->
        grammar
    end
  end
end
