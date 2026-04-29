defmodule CodingAdventures.MiniSqlite.Database do
  @moduledoc false

  alias CodingAdventures.MiniSqlite.Errors.{IntegrityError, OperationalError, ProgrammingError}
  alias CodingAdventures.SqlExecutionEngine
  alias CodingAdventures.SqlExecutionEngine.Errors.TableNotFoundError

  @row_id_column "__mini_sqlite_rowid"

  def start_link(opts) do
    autocommit = Keyword.get(opts, :autocommit, false)

    Agent.start_link(fn ->
      %{tables: %{}, snapshot: nil, autocommit: autocommit, closed: false}
    end)
  end

  def data_source(agent) do
    module_name = unique_module("DataSource")
    :persistent_term.put({__MODULE__, module_name, :agent}, agent)

    body =
      quote do
        @behaviour CodingAdventures.SqlExecutionEngine.DataSource

        @impl true
        def schema(table_name) do
          CodingAdventures.MiniSqlite.Database.schema_for_source(__MODULE__, table_name)
        end

        @impl true
        def scan(table_name) do
          CodingAdventures.MiniSqlite.Database.scan_for_source(__MODULE__, table_name)
        end
      end

    {:module, mod, _binary, _exports} =
      Module.create(module_name, body, Macro.Env.location(__ENV__))

    {:ok, mod}
  end

  def row_id_source(agent, table_name) do
    module_name = unique_module("RowIdSource")
    :persistent_term.put({__MODULE__, module_name, :agent}, agent)
    :persistent_term.put({__MODULE__, module_name, :table}, table_name)

    body =
      quote do
        @behaviour CodingAdventures.SqlExecutionEngine.DataSource

        @impl true
        def schema(table_name) do
          CodingAdventures.MiniSqlite.Database.schema_for_row_id_source(__MODULE__, table_name)
        end

        @impl true
        def scan(table_name) do
          CodingAdventures.MiniSqlite.Database.scan_for_row_id_source(__MODULE__, table_name)
        end
      end

    {:module, mod, _binary, _exports} =
      Module.create(module_name, body, Macro.Env.location(__ENV__))

    mod
  end

  def schema_for_source(source, table_name) do
    source
    |> source_agent()
    |> schema(table_name)
  end

  def scan_for_source(source, table_name) do
    source
    |> source_agent()
    |> scan(table_name)
  end

  def schema_for_row_id_source(source, table_name) do
    agent = source_agent(source)
    expected = :persistent_term.get({__MODULE__, source, :table})

    if normalize(table_name) != normalize(expected) do
      raise TableNotFoundError, table_name
    end

    schema(agent, table_name) ++ [@row_id_column]
  end

  def scan_for_row_id_source(source, table_name) do
    agent = source_agent(source)
    expected = :persistent_term.get({__MODULE__, source, :table})

    if normalize(table_name) != normalize(expected) do
      raise TableNotFoundError, table_name
    end

    agent
    |> scan(table_name)
    |> Enum.with_index()
    |> Enum.map(fn {row, index} -> Map.put(row, @row_id_column, index) end)
  end

  def schema(agent, table_name) do
    case Agent.get(agent, &schema_result(&1, table_name)) do
      {:ok, columns} -> columns
      {:error, :closed} -> raise ProgrammingError, "connection is closed"
      {:error, :missing} -> raise TableNotFoundError, table_name
    end
  end

  def scan(agent, table_name) do
    case Agent.get(agent, &scan_result(&1, table_name)) do
      {:ok, rows} -> rows
      {:error, :closed} -> raise ProgrammingError, "connection is closed"
      {:error, :missing} -> raise TableNotFoundError, table_name
    end
  end

  def assert_open(agent) do
    Agent.get(agent, fn state ->
      if state.closed do
        {:error, %ProgrammingError{message: "connection is closed"}}
      else
        :ok
      end
    end)
  end

  def begin(agent) do
    mutate(agent, fn state ->
      {empty_result(), ensure_snapshot(state)}
    end)
  end

  def commit(agent) do
    update_state(agent, fn state -> %{state | snapshot: nil} end)
  end

  def commit_result(agent) do
    mutate(agent, fn state -> {empty_result(), %{state | snapshot: nil}} end)
  end

  def rollback(agent) do
    update_state(agent, &restore_snapshot/1)
  end

  def rollback_result(agent) do
    mutate(agent, fn state -> {empty_result(), restore_snapshot(state)} end)
  end

  def close(agent) do
    update_state(agent, fn state ->
      state
      |> restore_snapshot()
      |> Map.put(:closed, true)
    end)
  end

  def create(agent, statement) do
    mutate(agent, fn state ->
      state = ensure_snapshot(state)
      key = normalize(statement.table)

      cond do
        Map.has_key?(state.tables, key) and statement.if_not_exists ->
          {empty_result(), state}

        Map.has_key?(state.tables, key) ->
          {%OperationalError{message: "table already exists: #{statement.table}"}, state}

        statement.columns == [] ->
          {%ProgrammingError{message: "CREATE TABLE requires at least one column"}, state}

        duplicate_column?(statement.columns) ->
          {%ProgrammingError{message: "duplicate column in CREATE TABLE"}, state}

        true ->
          table = %{columns: statement.columns, rows: []}
          {empty_result(), put_in(state.tables[key], table)}
      end
    end)
  end

  def drop(agent, statement) do
    mutate(agent, fn state ->
      state = ensure_snapshot(state)
      key = normalize(statement.table)

      cond do
        Map.has_key?(state.tables, key) ->
          {empty_result(), %{state | tables: Map.delete(state.tables, key)}}

        statement.if_exists ->
          {empty_result(), state}

        true ->
          {%OperationalError{message: "no such table: #{statement.table}"}, state}
      end
    end)
  end

  def insert(agent, statement) do
    mutate(agent, fn state ->
      state = ensure_snapshot(state)
      key = normalize(statement.table)

      case Map.get(state.tables, key) do
        nil ->
          {%OperationalError{message: "no such table: #{statement.table}"}, state}

        table ->
          with {:ok, columns} <- insert_columns(table, statement.columns),
               :ok <- validate_row_widths(statement.rows, columns) do
            rows =
              Enum.map(statement.rows, fn values ->
                base = Map.new(table.columns, &{&1, nil})

                columns
                |> Enum.zip(values)
                |> Enum.reduce(base, fn {col, val}, row -> Map.put(row, col, val) end)
              end)

            table = %{table | rows: table.rows ++ rows}
            {%{empty_result() | rows_affected: length(rows)}, put_in(state.tables[key], table)}
          else
            {:error, error} -> {error, state}
          end
      end
    end)
  end

  def update(agent, statement, row_ids) do
    mutate(agent, fn state ->
      state = ensure_snapshot(state)
      key = normalize(statement.table)

      case Map.get(state.tables, key) do
        nil ->
          {%OperationalError{message: "no such table: #{statement.table}"}, state}

        table ->
          with {:ok, assignments} <- canonical_assignments(table, statement.assignments) do
            id_set = MapSet.new(row_ids)

            rows =
              table.rows
              |> Enum.with_index()
              |> Enum.map(fn {row, index} ->
                if MapSet.member?(id_set, index) do
                  Enum.reduce(assignments, row, fn {col, val}, acc -> Map.put(acc, col, val) end)
                else
                  row
                end
              end)

            table = %{table | rows: rows}

            {%{empty_result() | rows_affected: MapSet.size(id_set)},
             put_in(state.tables[key], table)}
          else
            {:error, error} -> {error, state}
          end
      end
    end)
  end

  def delete(agent, statement, row_ids) do
    mutate(agent, fn state ->
      state = ensure_snapshot(state)
      key = normalize(statement.table)

      case Map.get(state.tables, key) do
        nil ->
          {%OperationalError{message: "no such table: #{statement.table}"}, state}

        table ->
          id_set = MapSet.new(row_ids)

          rows =
            table.rows
            |> Enum.with_index()
            |> Enum.reject(fn {_row, index} -> MapSet.member?(id_set, index) end)
            |> Enum.map(fn {row, _index} -> row end)

          table = %{table | rows: rows}

          {%{empty_result() | rows_affected: MapSet.size(id_set)},
           put_in(state.tables[key], table)}
      end
    end)
  end

  def matching_row_ids(agent, table_name, where_sql) do
    if String.trim(where_sql) == "" do
      Agent.get(agent, fn state ->
        case Map.get(state.tables, normalize(table_name)) do
          nil -> {:error, %OperationalError{message: "no such table: #{table_name}"}}
          table -> {:ok, Enum.to_list(0..(length(table.rows) - 1)) |> Enum.reject(&(&1 < 0))}
        end
      end)
    else
      source = row_id_source(agent, table_name)

      case SqlExecutionEngine.execute(
             "SELECT #{@row_id_column} FROM #{table_name} WHERE #{where_sql}",
             source
           ) do
        {:ok, result} ->
          {:ok, Enum.map(result.rows, &hd/1)}

        {:error, "Parse error:" <> _ = message} ->
          {:error, %ProgrammingError{message: message}}

        {:error, message} ->
          {:error, %OperationalError{message: message}}
      end
    end
  end

  defp mutate(agent, fun) do
    Agent.get_and_update(agent, fn state ->
      if state.closed do
        error = %ProgrammingError{message: "connection is closed"}
        {{:error, error}, state}
      else
        case fun.(state) do
          {%_{} = error, next_state} -> {{:error, error}, next_state}
          {result, next_state} -> {{:ok, result}, next_state}
        end
      end
    end)
  end

  defp update_state(agent, fun) do
    Agent.get_and_update(agent, fn state ->
      if state.closed do
        error = %ProgrammingError{message: "connection is closed"}
        {{:error, error}, state}
      else
        {:ok, fun.(state)}
      end
    end)
  end

  defp ensure_snapshot(%{autocommit: false, snapshot: nil} = state) do
    %{state | snapshot: %{tables: state.tables}}
  end

  defp ensure_snapshot(state), do: state

  defp restore_snapshot(%{snapshot: %{tables: tables}} = state) do
    %{state | tables: tables, snapshot: nil}
  end

  defp restore_snapshot(state), do: state

  defp empty_result do
    %{columns: [], rows: [], rows_affected: 0, lastrowid: nil}
  end

  defp insert_columns(table, []) do
    {:ok, table.columns}
  end

  defp insert_columns(table, columns) do
    canonical_columns(table, columns)
  end

  defp validate_row_widths(rows, columns) do
    case Enum.find(rows, &(length(&1) != length(columns))) do
      nil ->
        :ok

      row ->
        {:error,
         %IntegrityError{message: "INSERT expected #{length(columns)} values, got #{length(row)}"}}
    end
  end

  defp canonical_assignments(table, assignments) do
    Enum.reduce_while(assignments, {:ok, []}, fn {column, value}, {:ok, acc} ->
      case canonical_column(table, column) do
        {:ok, canonical} -> {:cont, {:ok, [{canonical, value} | acc]}}
        {:error, error} -> {:halt, {:error, error}}
      end
    end)
    |> case do
      {:ok, assignments} -> {:ok, Enum.reverse(assignments)}
      error -> error
    end
  end

  defp canonical_columns(table, columns) do
    Enum.reduce_while(columns, {:ok, []}, fn column, {:ok, acc} ->
      case canonical_column(table, column) do
        {:ok, canonical} -> {:cont, {:ok, [canonical | acc]}}
        {:error, error} -> {:halt, {:error, error}}
      end
    end)
    |> case do
      {:ok, columns} -> {:ok, Enum.reverse(columns)}
      error -> error
    end
  end

  defp canonical_column(table, column) do
    normalized = normalize(column)

    case Enum.find(table.columns, &(normalize(&1) == normalized)) do
      nil -> {:error, %OperationalError{message: "no such column: #{column}"}}
      column -> {:ok, column}
    end
  end

  defp duplicate_column?(columns) do
    normalized = Enum.map(columns, &normalize/1)
    length(normalized) != length(Enum.uniq(normalized))
  end

  defp schema_result(%{closed: true}, _table_name), do: {:error, :closed}

  defp schema_result(state, table_name) do
    case Map.get(state.tables, normalize(table_name)) do
      nil -> {:error, :missing}
      table -> {:ok, table.columns}
    end
  end

  defp scan_result(%{closed: true}, _table_name), do: {:error, :closed}

  defp scan_result(state, table_name) do
    case Map.get(state.tables, normalize(table_name)) do
      nil -> {:error, :missing}
      table -> {:ok, table.rows}
    end
  end

  defp source_agent(source) do
    :persistent_term.get({__MODULE__, source, :agent})
  end

  defp unique_module(kind) do
    n = :erlang.unique_integer([:positive, :monotonic])
    :"Elixir.CodingAdventures.MiniSqlite.#{kind}._#{n}"
  end

  defp normalize(name), do: String.downcase(name)
end
