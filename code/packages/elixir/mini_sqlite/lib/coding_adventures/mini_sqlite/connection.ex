defmodule CodingAdventures.MiniSqlite.Connection do
  @moduledoc "Connection handle for an Agent-backed in-memory mini-sqlite database."

  alias CodingAdventures.MiniSqlite.{Binding, Cursor, Database, Sql}
  alias CodingAdventures.MiniSqlite.Errors.{OperationalError, ProgrammingError}
  alias CodingAdventures.SqlExecutionEngine

  defstruct [:agent, :source]

  @type t :: %__MODULE__{}

  def open(opts) do
    autocommit = Keyword.get(opts, :autocommit, false)

    with {:ok, agent} <- Database.start_link(autocommit: autocommit),
         {:ok, source} <- Database.data_source(agent) do
      {:ok, %__MODULE__{agent: agent, source: source}}
    end
  end

  def cursor(%__MODULE__{} = connection) do
    with :ok <- Database.assert_open(connection.agent) do
      {:ok, %Cursor{connection: connection}}
    end
  end

  def execute(%__MODULE__{} = connection, sql, params \\ []) do
    with {:ok, cursor} <- cursor(connection) do
      Cursor.execute(cursor, sql, params)
    end
  end

  def executemany(%__MODULE__{} = connection, sql, params_seq) do
    with {:ok, cursor} <- cursor(connection) do
      Cursor.executemany(cursor, sql, params_seq)
    end
  end

  def commit(%__MODULE__{} = connection), do: Database.commit(connection.agent)
  def rollback(%__MODULE__{} = connection), do: Database.rollback(connection.agent)
  def close(%__MODULE__{} = connection), do: Database.close(connection.agent)

  @doc false
  def execute_bound(%__MODULE__{} = connection, sql, params) do
    with :ok <- Database.assert_open(connection.agent),
         {:ok, bound} <- Binding.bind(sql, params) do
      dispatch(connection, bound)
    end
  end

  defp dispatch(connection, sql) do
    case first_keyword(sql) do
      "BEGIN" ->
        Database.begin(connection.agent)

      "COMMIT" ->
        Database.commit_result(connection.agent)

      "ROLLBACK" ->
        Database.rollback_result(connection.agent)

      "SELECT" ->
        select(connection, sql)

      "CREATE" ->
        with {:ok, statement} <- Sql.parse_create(sql),
             do: Database.create(connection.agent, statement)

      "DROP" ->
        with {:ok, statement} <- Sql.parse_drop(sql),
             do: Database.drop(connection.agent, statement)

      "INSERT" ->
        with {:ok, statement} <- Sql.parse_insert(sql),
             do: Database.insert(connection.agent, statement)

      "UPDATE" ->
        with {:ok, statement} <- Sql.parse_update(sql),
             {:ok, row_ids} <-
               Database.matching_row_ids(connection.agent, statement.table, statement.where_sql) do
          Database.update(connection.agent, statement, row_ids)
        end

      "DELETE" ->
        with {:ok, statement} <- Sql.parse_delete(sql),
             {:ok, row_ids} <-
               Database.matching_row_ids(connection.agent, statement.table, statement.where_sql) do
          Database.delete(connection.agent, statement, row_ids)
        end

      _ ->
        {:error, %ProgrammingError{message: "unsupported SQL statement"}}
    end
  end

  defp select(connection, sql) do
    case SqlExecutionEngine.execute(sql, connection.source) do
      {:ok, result} ->
        {:ok,
         %{
           columns: result.columns,
           rows: result.rows,
           rows_affected: -1,
           lastrowid: nil
         }}

      {:error, "Parse error:" <> _ = message} ->
        {:error, %ProgrammingError{message: message}}

      {:error, message} ->
        {:error, %OperationalError{message: message}}
    end
  end

  defp first_keyword(sql) do
    sql
    |> String.trim_leading()
    |> String.split(~r/\s+/, parts: 2)
    |> hd()
    |> String.upcase()
  end
end
