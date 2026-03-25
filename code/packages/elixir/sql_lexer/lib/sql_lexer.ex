defmodule CodingAdventures.SqlLexer do
  @moduledoc """
  SQL Lexer — Thin wrapper around the grammar-driven lexer engine.

  This module reads `sql.tokens` from the shared grammars directory and
  uses `GrammarLexer.tokenize/2` to tokenize SQL source code. It's the
  Elixir equivalent of the Python `sql_lexer` package.

  ## Usage

      {:ok, tokens} = CodingAdventures.SqlLexer.tokenize_sql("SELECT id FROM users")

  ## How It Works

  1. `create_sql_lexer/1` parses `sql.tokens` into a `TokenGrammar` struct.
     The default path points to the shared grammars directory, but you can
     pass a custom path for testing or alternative grammar files.

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

  # The shared grammars directory lives four levels above this file:
  #   lib/sql_lexer.ex
  #     ↑ lib
  #       ↑ sql_lexer
  #         ↑ elixir
  #           ↑ packages
  #             ↑ code  (the repo root's code/ directory)
  #               grammars/
  @default_grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "grammars"])
                        |> Path.expand()

  @doc """
  Create (parse) the SQL lexer grammar from a `.tokens` file.

  Accepts an optional `grammars_dir` path so tests can point at a custom
  directory.  In normal use you never need to pass the argument — it
  defaults to the shared `code/grammars` directory.

  Returns `{:ok, grammar}` on success, `{:error, message}` on failure.

  ## Example

      {:ok, grammar} = CodingAdventures.SqlLexer.create_sql_lexer()
      grammar.case_insensitive  # => true
  """
  @spec create_sql_lexer(String.t() | nil) ::
          {:ok, TokenGrammar.t()} | {:error, String.t()}
  def create_sql_lexer(grammars_dir \\ nil) do
    dir = grammars_dir || @default_grammars_dir
    tokens_path = Path.join(dir, "sql.tokens")

    case File.read(tokens_path) do
      {:ok, text} ->
        TokenGrammar.parse(text)

      {:error, reason} ->
        {:error, "Cannot read sql.tokens: #{:file.format_error(reason)}"}
    end
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
