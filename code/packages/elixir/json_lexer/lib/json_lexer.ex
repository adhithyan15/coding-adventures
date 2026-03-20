defmodule CodingAdventures.JsonLexer do
  @moduledoc """
  JSON Lexer — Thin wrapper around the grammar-driven lexer engine.

  This module reads `json.tokens` from the shared grammars directory and
  uses `GrammarLexer.tokenize/2` to tokenize JSON source code. It's the
  Elixir equivalent of the Python `json_lexer` package.

  ## Usage

      {:ok, tokens} = CodingAdventures.JsonLexer.tokenize(~s({"key": 42}))

  ## How It Works

  1. On first call, `create_lexer/0` parses `json.tokens` into a
     `TokenGrammar` struct. This is cached in a module attribute-style
     persistent term.

  2. `tokenize/1` passes the source and grammar to `GrammarLexer.tokenize/2`,
     which does all the real work.

  The entire module is about 20 lines of actual logic — the grammar file
  does the heavy lifting.
  """

  alias CodingAdventures.GrammarTools.TokenGrammar
  alias CodingAdventures.Lexer.GrammarLexer

  @grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "grammars"])
                |> Path.expand()

  @doc """
  Tokenize JSON source code.

  Returns `{:ok, tokens}` on success, `{:error, message}` on failure.
  Each token is a `%Token{type, value, line, column}` struct.
  """
  @spec tokenize(String.t()) :: {:ok, [CodingAdventures.Lexer.Token.t()]} | {:error, String.t()}
  def tokenize(source) do
    grammar = get_grammar()
    GrammarLexer.tokenize(source, grammar)
  end

  @doc """
  Parse the json.tokens grammar file and return the TokenGrammar.

  This is useful if you want to inspect the grammar or reuse it directly.
  """
  @spec create_lexer() :: TokenGrammar.t()
  def create_lexer do
    tokens_path = Path.join(@grammars_dir, "json.tokens")
    {:ok, grammar} = TokenGrammar.parse(File.read!(tokens_path))
    grammar
  end

  # Cache the grammar in a persistent_term for fast repeated access.
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
