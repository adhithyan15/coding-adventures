defmodule CodingAdventures.SqlLexer do
  @moduledoc """
  SQL Lexer — Thin wrapper around the grammar-driven lexer engine.

  This module reads `sql.tokens` from the shared grammars directory and
  uses `GrammarLexer.tokenize/2` to tokenize SQL source code. It's the
  Elixir equivalent of the Python `sql_lexer` package.

  ## Usage

      {:ok, tokens} = CodingAdventures.SqlLexer.tokenize_sql("SELECT id FROM users")

  ## How It Works

  1. `create_sql_lexer/0` returns the `TokenGrammar` struct embedded as
     native Elixir data in `CodingAdventures.SqlLexer.Grammar`, compiled
     ahead of time from `sql.tokens` by `grammar-tools compile-tokens`.

  2. `tokenize_sql/1` uses a cached grammar (via `persistent_term`) and
     delegates to `GrammarLexer.tokenize/2`.

  ## Case-Insensitive Keyword Matching

  The SQL grammar sets `@case_insensitive true` in `sql.tokens`. This means
  keyword values are automatically normalized to uppercase by the lexer
  engine. So `select`, `SELECT`, and `Select` all produce a KEYWORD token
  with value `"SELECT"`.

  ## Token Types

  - `KEYWORD` — SQL reserved words (SELECT, FROM, WHERE, …) in uppercase
  - `NAME` — Identifiers (`table_name`, `column`, `` `quoted_id` ``)
  - `NUMBER` — Integer or decimal literals (`42`, `3.14`)
  - `STRING` — Single-quoted string literals (quotes stripped)
  - `EQUALS`, `NOT_EQUALS`, `LESS_THAN`, `GREATER_THAN` — comparison ops
  - `LESS_EQUALS`, `GREATER_EQUALS` — multi-character comparison ops
  - `PLUS`, `MINUS`, `STAR`, `SLASH`, `PERCENT` — arithmetic ops
  - `LPAREN`, `RPAREN`, `COMMA`, `SEMICOLON`, `DOT` — punctuation
  - `EOF` — end of input

  Comments (`-- …` and `/* … */`) and whitespace are skipped silently.
  """

  alias CodingAdventures.GrammarTools.TokenGrammar
  alias CodingAdventures.Lexer.GrammarLexer
  alias CodingAdventures.SqlLexer.Grammar, as: GrammarSource

  @doc """
  Return the SQL lexer grammar.

  Kept as an `{:ok, grammar}` tuple for backwards compatibility with
  existing callers, even though the compiled grammar module can no longer
  fail the way a disk read could.

  ## Example

      {:ok, grammar} = CodingAdventures.SqlLexer.create_sql_lexer()
      grammar.case_insensitive  # => true
  """
  @spec create_sql_lexer() :: {:ok, TokenGrammar.t()}
  def create_sql_lexer do
    {:ok, GrammarSource.token_grammar()}
  end

  @doc """
  Tokenize SQL source code.

  Returns `{:ok, tokens}` on success, `{:error, message}` on failure.
  Each token is a `%Token{type, value, line, column}` struct.

  Keywords are normalized to uppercase regardless of how they were typed:
  `"select"`, `"SELECT"`, and `"Select"` all produce
  `%Token{type: "KEYWORD", value: "SELECT"}`.

  ## Examples

      iex> {:ok, tokens} = CodingAdventures.SqlLexer.tokenize_sql("SELECT 1")
      iex> hd(tokens).type
      "KEYWORD"
      iex> hd(tokens).value
      "SELECT"
  """
  @spec tokenize_sql(String.t()) ::
          {:ok, [CodingAdventures.Lexer.Token.t()]} | {:error, String.t()}
  def tokenize_sql(source) do
    grammar = get_grammar()
    GrammarLexer.tokenize(source, grammar)
  end

  # ---------------------------------------------------------------------------
  # Grammar caching
  # ---------------------------------------------------------------------------
  #
  # We cache the parsed grammar in a persistent_term keyed by this module.
  # persistent_term survives across function calls and is JIT-compiled to a
  # constant, making repeated calls to tokenize_sql/1 very fast.

  defp get_grammar do
    case :persistent_term.get({__MODULE__, :grammar}, nil) do
      nil ->
        {:ok, grammar} = create_sql_lexer()
        :persistent_term.put({__MODULE__, :grammar}, grammar)
        grammar

      grammar ->
        grammar
    end
  end
end
