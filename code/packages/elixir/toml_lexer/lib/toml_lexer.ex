defmodule CodingAdventures.TomlLexer do
  @moduledoc """
  TOML Lexer — Thin wrapper around the grammar-driven lexer engine.

  This module reads `toml.tokens` from the shared grammars directory and
  uses `GrammarLexer.tokenize/2` to tokenize TOML source code. It's the
  Elixir equivalent of the Python `toml_lexer` package.

  ## Usage

      {:ok, tokens} = CodingAdventures.TomlLexer.tokenize(~s(title = "TOML Example"))

  ## How It Works

  1. On first call, `create_lexer/0` parses `toml.tokens` into a
     `TokenGrammar` struct. This is cached via `persistent_term` for fast
     repeated access.

  2. `tokenize/1` passes the source and grammar to `GrammarLexer.tokenize/2`,
     which does all the real work.

  ## TOML-Specific Notes

  TOML is newline-sensitive — key-value pairs are delimited by newlines —
  so the lexer emits NEWLINE tokens. The skip pattern only covers spaces,
  tabs, and comments (not newlines).

  TOML has four string types (basic, literal, multi-line basic, multi-line
  literal) with different escape semantics. The `toml.tokens` grammar uses
  `escapes: none` to tell the lexer to strip quotes but leave escape
  sequences as raw text. The TOML parser's semantic layer handles
  type-specific escape processing.
  """

  alias CodingAdventures.GrammarTools.TokenGrammar
  alias CodingAdventures.Lexer.GrammarLexer

  @grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "grammars"])
                |> Path.expand()

  @doc """
  Tokenize TOML source code.

  Returns `{:ok, tokens}` on success, `{:error, message}` on failure.
  Each token is a `%Token{type, value, line, column}` struct.
  """
  @spec tokenize(String.t()) :: {:ok, [CodingAdventures.Lexer.Token.t()]} | {:error, String.t()}
  def tokenize(source) do
    grammar = get_grammar()
    GrammarLexer.tokenize(source, grammar)
  end

  @doc """
  Parse the toml.tokens grammar file and return the TokenGrammar.

  This is useful if you want to inspect the grammar or reuse it directly.
  """
  @spec create_lexer() :: TokenGrammar.t()
  def create_lexer do
    tokens_path = Path.join(@grammars_dir, "toml.tokens")
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
