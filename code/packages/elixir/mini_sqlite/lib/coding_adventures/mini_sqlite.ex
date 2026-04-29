defmodule CodingAdventures.MiniSqlite do
  @moduledoc """
  Level 0 mini-sqlite facade for Elixir.

  The package is intentionally small: it provides an in-memory database,
  qmark parameter binding, DB-API-inspired connection/cursor helpers, and
  delegates `SELECT` statements to `CodingAdventures.SqlExecutionEngine`.
  """

  alias CodingAdventures.MiniSqlite.Connection
  alias CodingAdventures.MiniSqlite.Errors.NotSupportedError

  @apilevel "2.0"
  @threadsafety 1
  @paramstyle "qmark"

  def apilevel, do: @apilevel
  def threadsafety, do: @threadsafety
  def paramstyle, do: @paramstyle

  @spec connect(String.t(), keyword()) :: {:ok, Connection.t()} | {:error, Exception.t()}
  def connect(database, opts \\ [])

  def connect(":memory:", opts) do
    Connection.open(opts)
  end

  def connect(_database, _opts) do
    {:error, %NotSupportedError{message: "Elixir mini-sqlite supports only :memory: in Level 0"}}
  end

  defdelegate cursor(connection), to: Connection
  defdelegate execute(connection, sql, params \\ []), to: Connection
  defdelegate executemany(connection, sql, params_seq), to: Connection
  defdelegate commit(connection), to: Connection
  defdelegate rollback(connection), to: Connection
  defdelegate close(connection), to: Connection
end
