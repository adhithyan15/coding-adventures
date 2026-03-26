defmodule CodingAdventures.SqlExecutionEngine do
  @moduledoc """
  SELECT-only SQL execution engine.

  This module is the public API for the SQL execution engine.  It combines:

  - `CodingAdventures.SqlParser` — parses a SQL string into an AST
  - `Executor` — walks the AST and runs the query pipeline
  - `DataSource` — a behaviour for pluggable data sources

  ## Quick start

      defmodule MySource do
        @behaviour CodingAdventures.SqlExecutionEngine.DataSource

        @impl true
        def schema("users"), do: ["id", "name", "email"]

        @impl true
        def scan("users") do
          [
            %{"id" => 1, "name" => "Alice", "email" => "alice@example.com"},
            %{"id" => 2, "name" => "Bob",   "email" => "bob@example.com"},
          ]
        end
      end

      {:ok, result} = SqlExecutionEngine.execute("SELECT * FROM users", MySource)
      result.columns  # => ["id", "name", "email"]
      result.rows     # => [[1, "Alice", "alice@example.com"], [2, "Bob", "bob@example.com"]]

  ## Supported SQL

  - `SELECT ... FROM ... [WHERE ...]`
  - `SELECT ... FROM ... [INNER/LEFT/RIGHT/FULL/CROSS] JOIN ... ON ...`
  - `SELECT ... FROM ... GROUP BY ... [HAVING ...]`
  - `SELECT ... FROM ... ORDER BY ... [ASC|DESC]`
  - `SELECT ... FROM ... LIMIT n [OFFSET m]`
  - `SELECT DISTINCT ...`
  - Column aliases: `SELECT name AS employee_name`
  - Table aliases: `FROM employees AS e`
  - Expressions: arithmetic, comparisons, `BETWEEN`, `IN`, `LIKE`, `IS NULL`
  - Aggregates: `COUNT(*)`, `COUNT(col)`, `SUM`, `AVG`, `MIN`, `MAX`
  - Three-valued NULL logic in WHERE / HAVING

  ## Error handling

  Parse errors return `{:error, message}`.
  Runtime errors (unknown table, unknown column) raise typed exceptions:

  - `TableNotFoundError` — table not registered in DataSource
  - `ColumnNotFoundError` — column not in any row context
  - `UnsupportedQueryError` — non-SELECT statement
  """

  alias CodingAdventures.SqlParser
  alias CodingAdventures.SqlExecutionEngine.{Executor, Result}

  @doc """
  Parse and execute a SQL SELECT statement.

  Returns `{:ok, %QueryResult{columns:, rows:}}` on success.
  Returns `{:error, message}` if the SQL cannot be parsed or the statement
  is not a SELECT.

  ## Parameters
  - `sql`         — SQL string to execute
  - `data_source` — module implementing the `DataSource` behaviour

  ## Examples

      iex> {:ok, r} = SqlExecutionEngine.execute("SELECT * FROM employees", InMemorySource)
      iex> r.columns
      ["id", "name", "dept_id", "salary", "active"]
      iex> length(r.rows)
      4
  """
  @spec execute(String.t(), module()) :: {:ok, Result.t()} | {:error, String.t()}
  def execute(sql, data_source) when is_binary(sql) do
    case SqlParser.parse_sql(sql) do
      {:ok, ast} ->
        try do
          Executor.execute(ast, data_source)
        rescue
          e -> {:error, Exception.message(e)}
        end

      {:error, msg} ->
        {:error, "Parse error: #{msg}"}
    end
  end

  @doc """
  Parse and execute multiple SQL SELECT statements separated by `;`.

  Returns `{:ok, [%QueryResult{}, ...]}` — one result per statement.

  ## Parameters
  - `sql`         — SQL string containing one or more `;`-separated statements
  - `data_source` — module implementing the `DataSource` behaviour
  """
  @spec execute_all(String.t(), module()) :: {:ok, [Result.t()]} | {:error, String.t()}
  def execute_all(sql, data_source) when is_binary(sql) do
    case SqlParser.parse_sql(sql) do
      {:ok, ast} ->
        try do
          Executor.execute_all(ast, data_source)
        rescue
          e -> {:error, Exception.message(e)}
        end

      {:error, msg} ->
        {:error, "Parse error: #{msg}"}
    end
  end
end
