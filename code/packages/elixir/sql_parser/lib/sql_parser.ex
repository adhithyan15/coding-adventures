defmodule CodingAdventures.SqlParser do
  @moduledoc """
  SQL Parser — Thin wrapper around the grammar-driven parser engine.

  This module combines `SqlLexer.tokenize_sql/1` with
  `GrammarParser.parse/2` to parse SQL source code into an AST.  It reads
  `sql.grammar` from the shared grammars directory.

  ## Usage

      {:ok, ast} = CodingAdventures.SqlParser.parse_sql("SELECT id FROM users")

  The returned AST is a tree of `ASTNode` structs where `rule_name`
  indicates the grammar rule matched (e.g., `"program"`, `"select_stmt"`,
  `"expr"`) and `children` contains sub-nodes and tokens.

  ## Supported SQL Statements

  The grammar covers an ANSI SQL subset:

  - `SELECT … FROM … [WHERE …] [GROUP BY …] [HAVING …] [ORDER BY …] [LIMIT …]`
  - `INSERT INTO … VALUES (…)`
  - `UPDATE … SET …`
  - `DELETE FROM …`
  - `CREATE TABLE … (…)`
  - `DROP TABLE …`

  Multiple statements separated by `;` are all parsed under the root
  `"program"` node.

  ## Case-Insensitive Keywords

  The SQL lexer normalizes all keyword values to uppercase (see `SqlLexer`).
  The grammar matches quoted keyword literals like `"SELECT"` against the
  already-uppercase token values, so keyword case in source is irrelevant.
  """

  alias CodingAdventures.GrammarTools.ParserGrammar
  alias CodingAdventures.SqlLexer
  alias CodingAdventures.Parser.{GrammarParser, ASTNode}

  # The shared grammars directory lives four levels above this file:
  #   lib/sql_parser.ex
  #     ↑ lib
  #       ↑ sql_parser
  #         ↑ elixir
  #           ↑ packages
  #             ↑ code
  #               grammars/
  @default_grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "grammars"])
                        |> Path.expand()

  @doc """
  Parse SQL source code into an AST.

  Returns `{:ok, ast_node}` on success, `{:error, message}` on failure.

  The root node always has `rule_name == "program"` (as specified by the
  sql.grammar entry point).

  ## Examples

      iex> {:ok, node} = CodingAdventures.SqlParser.parse_sql("SELECT 1 FROM t")
      iex> node.rule_name
      "program"
  """
  @spec parse_sql(String.t()) :: {:ok, ASTNode.t()} | {:error, String.t()}
  def parse_sql(source) do
    grammar = get_grammar()

    case SqlLexer.tokenize_sql(source) do
      {:ok, tokens} -> GrammarParser.parse(tokens, grammar)
      {:error, msg} -> {:error, msg}
    end
  end

  @doc """
  Parse the sql.grammar file and return the `ParserGrammar`.

  Useful for inspecting the grammar or running the parser directly.
  Accepts an optional `grammars_dir` path for testing.

  Returns `{:ok, grammar}` on success, `{:error, message}` on failure.
  """
  @spec create_sql_parser(String.t() | nil) ::
          {:ok, ParserGrammar.t()} | {:error, String.t()}
  def create_sql_parser(grammars_dir \\ nil) do
    dir = grammars_dir || @default_grammars_dir
    grammar_path = Path.join(dir, "sql.grammar")

    case File.read(grammar_path) do
      {:ok, text} ->
        ParserGrammar.parse(text)

      {:error, reason} ->
        {:error, "Cannot read sql.grammar: #{:file.format_error(reason)}"}
    end
  end

  # ---------------------------------------------------------------------------
  # Grammar caching
  # ---------------------------------------------------------------------------
  #
  # Parsing the grammar file takes a non-trivial amount of time on the first
  # call.  We cache the result in a persistent_term keyed by this module so
  # subsequent calls to parse_sql/1 are essentially free.

  defp get_grammar do
    case :persistent_term.get({__MODULE__, :grammar}, nil) do
      nil ->
        {:ok, grammar} = create_sql_parser()
        :persistent_term.put({__MODULE__, :grammar}, grammar)
        grammar

      grammar ->
        grammar
    end
  end
end
