defmodule CodingAdventures.SqlExecutionEngine.Errors do
  @moduledoc """
  Error types for the SQL execution engine.

  The engine raises typed exceptions rather than returning error tuples for
  most runtime errors.  This keeps the execution pipeline clean — the happy
  path is a straight pipeline with no {:ok, _} / {:error, _} threading —
  and lets callers catch specific error types if they need to distinguish
  them.

  ## Error hierarchy

      TableNotFoundError   — referenced table is not registered in the DataSource
      ColumnNotFoundError  — referenced column does not exist in any row context
      UnsupportedQueryError — a SQL construct is outside the SELECT-only scope
      ExecutionError        — catch-all for unexpected failures during execution

  ## Usage pattern

      try do
        SqlExecutionEngine.execute(sql, source)
      rescue
        e in TableNotFoundError -> {:error, :table_not_found, e.message}
        e in ColumnNotFoundError -> {:error, :column_not_found, e.message}
      end
  """

  # ---------------------------------------------------------------------------
  # TableNotFoundError
  # ---------------------------------------------------------------------------
  #
  # Raised when a SQL query references a table name that is not known to the
  # DataSource.  This can happen in the FROM clause or in JOIN targets.
  #
  # Example: SELECT * FROM non_existent
  #   ↳ raises TableNotFoundError, "Table not found: non_existent"

  defmodule TableNotFoundError do
    @moduledoc "Raised when a referenced table does not exist in the DataSource."

    defexception [:message]

    @impl true
    def exception(table_name) when is_binary(table_name) do
      %__MODULE__{message: "Table not found: #{table_name}"}
    end

    def exception(message), do: %__MODULE__{message: message}
  end

  # ---------------------------------------------------------------------------
  # ColumnNotFoundError
  # ---------------------------------------------------------------------------
  #
  # Raised when an expression references a column name that is not present in
  # any row in the current row context map.
  #
  # The row context is a flat map:  %{"table.col" => value, "col" => value, …}
  # (Both qualified "table.col" and bare "col" are stored so that expressions
  # can use either form.)
  #
  # Example: SELECT unknown_col FROM employees
  #   ↳ raises ColumnNotFoundError, "Column not found: unknown_col"

  defmodule ColumnNotFoundError do
    @moduledoc "Raised when a referenced column does not exist in the row context."

    defexception [:message]

    @impl true
    def exception(col_name) when is_binary(col_name) do
      %__MODULE__{message: "Column not found: #{col_name}"}
    end

    def exception(message), do: %__MODULE__{message: message}
  end

  # ---------------------------------------------------------------------------
  # UnsupportedQueryError
  # ---------------------------------------------------------------------------
  #
  # This engine only executes SELECT statements.  Attempting to run INSERT,
  # UPDATE, DELETE, CREATE TABLE, or DROP TABLE raises this error.
  #
  # Why SELECT-only?  The DataSource behaviour models a read-only data layer —
  # think an in-memory table, a CSV file, or an HTTP API.  Write operations
  # would need a separate behaviour contract.

  defmodule UnsupportedQueryError do
    @moduledoc "Raised when a non-SELECT SQL statement is passed to the engine."

    defexception [:message]

    @impl true
    def exception(stmt_type) when is_binary(stmt_type) do
      %__MODULE__{message: "Unsupported statement type: #{stmt_type}. Only SELECT is supported."}
    end

    def exception(message), do: %__MODULE__{message: message}
  end

  # ---------------------------------------------------------------------------
  # ExecutionError
  # ---------------------------------------------------------------------------
  #
  # Catch-all for unexpected failures during query execution — for example,
  # type errors in arithmetic expressions or malformed AST nodes that were
  # not caught during parsing.

  defmodule ExecutionError do
    @moduledoc "Raised for unexpected errors during SQL execution."

    defexception [:message]

    @impl true
    def exception(message) when is_binary(message) do
      %__MODULE__{message: "Execution error: #{message}"}
    end

    def exception(message), do: %__MODULE__{message: inspect(message)}
  end
end
