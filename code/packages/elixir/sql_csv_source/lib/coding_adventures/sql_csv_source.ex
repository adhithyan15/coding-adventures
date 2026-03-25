defmodule CodingAdventures.SqlCsvSource do
  @moduledoc """
  SQL-over-CSV: run SELECT queries against a directory of CSV files.

  This is the public API module for the `sql_csv_source` package.  It provides
  two convenience functions:

  - `new/1` — create a `DataSource` module pointing at a CSV directory.
  - `execute/2` — parse and run a SQL query against that source.

  For more control (e.g. passing the source to `SqlExecutionEngine.execute_all/2`)
  use `CodingAdventures.SqlCsvSource.CsvDataSource.new/1` directly — it returns
  the same module atom.

  ## Stack position

      sql_execution_engine  ← SQL AST → result pipeline
              │
              │  DataSource behaviour (module atom dispatch)
              ▼
       CsvDataSource         ← this adapter (reads files, coerces types)
              │
              │  CsvParser.parse_csv/1
              ▼
           csv_parser        ← raw CSV → list of string-valued maps

  ## How `new/1` works

  The SQL execution engine dispatches data source calls as:

      data_source.schema(table_name)
      data_source.scan(table_name)

  …where `data_source` is a module atom.  To bridge runtime directory
  configuration with this module-based dispatch, `new/1` uses `Module.create/3`
  to build a fresh anonymous module at runtime with the directory path baked in
  as a module attribute.  Each call to `new/1` returns a distinct module atom.

  ## Example

      source = CodingAdventures.SqlCsvSource.new("path/to/csvdir")

      {:ok, result} = CodingAdventures.SqlCsvSource.execute(
        "SELECT e.name, d.name " <>
        "FROM employees AS e " <>
        "INNER JOIN departments AS d ON e.dept_id = d.id",
        source
      )

      result.columns  #=> ["e.name", "d.name"]
      result.rows     #=> [["Alice", "Engineering"], ["Bob", "Marketing"],
      #                     ["Carol", "Engineering"]]
  """

  alias CodingAdventures.SqlCsvSource.CsvDataSource
  alias CodingAdventures.SqlExecutionEngine

  @doc """
  Create a `DataSource` module pointing at `dir`.

  `dir` must be a path to a directory containing CSV files named
  `<tablename>.csv`.  The directory is not read at construction time — files
  are opened lazily when a query references their table.

  Returns a module atom implementing the `DataSource` behaviour.  This atom
  can be passed directly to `SqlExecutionEngine.execute/2` or to the
  convenience function `execute/2` in this module.

  ## Example

      source = CodingAdventures.SqlCsvSource.new("test/fixtures")
      # => :"Elixir.CodingAdventures.SqlCsvSource.CsvDataSource._N"
  """
  @spec new(String.t()) :: module()
  def new(dir) when is_binary(dir) do
    CsvDataSource.new(dir)
  end

  @doc """
  Execute a SQL SELECT statement against a CSV-backed data source.

  This is a convenience wrapper around `SqlExecutionEngine.execute/2`.
  Pass a source module created by `new/1` (or `CsvDataSource.new/1`).

  Returns `{:ok, %QueryResult{columns:, rows:}}` on success, or
  `{:error, message}` if the SQL cannot be parsed.

  Runtime errors (unknown table, unknown column) are caught internally by the
  execution engine and returned as `{:error, message}` tuples.

  ## Parameters

  - `sql`    — SQL SELECT string to execute
  - `source` — module atom returned by `new/1`

  ## Example

      source = CodingAdventures.SqlCsvSource.new("test/fixtures")

      {:ok, result} = CodingAdventures.SqlCsvSource.execute(
        "SELECT * FROM departments",
        source
      )

      result.columns  #=> ["id", "name", "budget"]
      result.rows     #=> [[1, "Engineering", 500000], [2, "Marketing", 200000]]
  """
  @spec execute(String.t(), module()) ::
          {:ok, SqlExecutionEngine.Result.t()} | {:error, String.t()}
  def execute(sql, source) when is_binary(sql) do
    SqlExecutionEngine.execute(sql, source)
  end
end
