defmodule CodingAdventures.MiniSqlite.Cursor do
  @moduledoc "Immutable cursor state for mini-sqlite query results."

  alias CodingAdventures.MiniSqlite.Connection
  alias CodingAdventures.MiniSqlite.Errors.ProgrammingError

  defstruct connection: nil,
            description: [],
            rowcount: -1,
            lastrowid: nil,
            arraysize: 1,
            rows: [],
            offset: 0,
            closed: false

  @type t :: %__MODULE__{}

  def execute(cursor, sql, params \\ [])

  def execute(%__MODULE__{closed: true}, _sql, _params) do
    {:error, %ProgrammingError{message: "cursor is closed"}}
  end

  def execute(%__MODULE__{} = cursor, sql, params) do
    case Connection.execute_bound(cursor.connection, sql, params) do
      {:ok, result} ->
        {:ok,
         %{
           cursor
           | description: Enum.map(result.columns, &%{name: &1}),
             rowcount: result.rows_affected,
             lastrowid: result.lastrowid,
             rows: result.rows,
             offset: 0
         }}

      {:error, error} ->
        {:error, error}
    end
  end

  def executemany(%__MODULE__{closed: true}, _sql, _params_seq) do
    {:error, %ProgrammingError{message: "cursor is closed"}}
  end

  def executemany(%__MODULE__{} = cursor, sql, params_seq) do
    Enum.reduce_while(params_seq, {:ok, cursor, 0}, fn params, {:ok, cur, total} ->
      case execute(cur, sql, params) do
        {:ok, next} ->
          total = if next.rowcount > 0, do: total + next.rowcount, else: total
          {:cont, {:ok, next, total}}

        {:error, error} ->
          {:halt, {:error, error}}
      end
    end)
    |> case do
      {:ok, cur, total} when params_seq != [] -> {:ok, %{cur | rowcount: total}}
      {:ok, cur, _total} -> {:ok, cur}
      {:error, error} -> {:error, error}
    end
  end

  def fetchone(%__MODULE__{closed: true} = cursor), do: {nil, cursor}

  def fetchone(%__MODULE__{} = cursor) do
    if cursor.offset >= length(cursor.rows) do
      {nil, cursor}
    else
      {Enum.at(cursor.rows, cursor.offset), %{cursor | offset: cursor.offset + 1}}
    end
  end

  def fetchmany(cursor, size \\ nil)

  def fetchmany(%__MODULE__{closed: true} = cursor, _size), do: {[], cursor}

  def fetchmany(%__MODULE__{} = cursor, size) do
    count = size || cursor.arraysize
    rows = cursor.rows |> Enum.drop(cursor.offset) |> Enum.take(count)
    {rows, %{cursor | offset: min(cursor.offset + count, length(cursor.rows))}}
  end

  def fetchall(%__MODULE__{closed: true} = cursor), do: {[], cursor}

  def fetchall(%__MODULE__{} = cursor) do
    rows = Enum.drop(cursor.rows, cursor.offset)
    {rows, %{cursor | offset: length(cursor.rows)}}
  end

  def close(%__MODULE__{} = cursor) do
    %{cursor | closed: true, rows: [], description: []}
  end
end
