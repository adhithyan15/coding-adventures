defmodule CodingAdventures.SqlBackend do
  @moduledoc """
  Storage backend contract helpers for the mini-sqlite Elixir port.
  """

  defmodule Blob do
    defstruct [:bytes]
  end

  defmodule ColumnDef do
    defstruct name: "",
              type_name: "",
              not_null: false,
              primary_key: false,
              unique: false,
              autoincrement: false,
              default: nil,
              has_default: false,
              check_expression: nil,
              foreign_key: nil

    def effective_not_null?(%__MODULE__{} = column), do: column.not_null or column.primary_key
    def effective_unique?(%__MODULE__{} = column), do: column.unique or column.primary_key
  end

  defmodule IndexDef do
    defstruct name: "", table: "", columns: [], unique: false, auto: false
  end

  defmodule TriggerDef do
    defstruct name: "", table: "", timing: "", event: "", body: ""
  end

  defmodule TableNotFound do
    defexception [:table, message: "table not found"]

    def exception(table),
      do: %__MODULE__{table: table, message: "table not found: #{inspect(table)}"}
  end

  defmodule TableAlreadyExists do
    defexception [:table, message: "table already exists"]

    def exception(table),
      do: %__MODULE__{table: table, message: "table already exists: #{inspect(table)}"}
  end

  defmodule ColumnNotFound do
    defexception [:table, :column, message: "column not found"]

    def exception({table, column}),
      do: %__MODULE__{
        table: table,
        column: column,
        message: "column not found: #{inspect(table)}.#{inspect(column)}"
      }
  end

  defmodule ColumnAlreadyExists do
    defexception [:table, :column, message: "column already exists"]

    def exception({table, column}),
      do: %__MODULE__{
        table: table,
        column: column,
        message: "column already exists: #{inspect(table)}.#{inspect(column)}"
      }
  end

  defmodule ConstraintViolation do
    defexception [:table, :column, message: "constraint violation"]

    def exception({table, column, message}),
      do: %__MODULE__{table: table, column: column, message: message}
  end

  defmodule Unsupported do
    defexception [:operation, message: "operation not supported"]

    def exception(operation),
      do: %__MODULE__{operation: operation, message: "operation not supported: #{operation}"}
  end

  defmodule Internal do
    defexception [:message]
  end

  defmodule IndexAlreadyExists do
    defexception [:index, message: "index already exists"]

    def exception(index),
      do: %__MODULE__{index: index, message: "index already exists: #{inspect(index)}"}
  end

  defmodule IndexNotFound do
    defexception [:index, message: "index not found"]

    def exception(index),
      do: %__MODULE__{index: index, message: "index not found: #{inspect(index)}"}
  end

  defmodule TriggerAlreadyExists do
    defexception [:name, message: "trigger already exists"]

    def exception(name),
      do: %__MODULE__{name: name, message: "trigger already exists: #{inspect(name)}"}
  end

  defmodule TriggerNotFound do
    defexception [:name, message: "trigger not found"]

    def exception(name),
      do: %__MODULE__{name: name, message: "trigger not found: #{inspect(name)}"}
  end

  def blob(bytes), do: %Blob{bytes: IO.iodata_to_binary(bytes)}

  def column_def(opts) do
    struct!(ColumnDef, Keyword.put_new(opts, :has_default, Keyword.has_key?(opts, :default)))
  end

  def index_def(opts), do: struct!(IndexDef, opts)

  def trigger_def(opts) do
    opts
    |> Keyword.update(:timing, "", &String.upcase(to_string(&1)))
    |> Keyword.update(:event, "", &String.upcase(to_string(&1)))
    |> then(&struct!(TriggerDef, &1))
  end

  def sql_value?(value),
    do:
      is_nil(value) or is_boolean(value) or is_integer(value) or is_float(value) or
        is_binary(value) or match?(%Blob{}, value)

  def type_name(nil), do: "NULL"
  def type_name(value) when is_boolean(value), do: "BOOLEAN"
  def type_name(value) when is_integer(value), do: "INTEGER"
  def type_name(value) when is_float(value), do: "REAL"
  def type_name(value) when is_binary(value), do: "TEXT"
  def type_name(%Blob{}), do: "BLOB"

  def compare(left, right) do
    cond do
      rank(left) < rank(right) -> -1
      rank(left) > rank(right) -> 1
      true -> compare_same_rank(left, right)
    end
  end

  def backend_as_schema_provider(backend) do
    %{
      columns: fn table -> backend.__struct__.columns(backend, table) |> Enum.map(& &1.name) end,
      list_indexes: fn table -> backend.__struct__.list_indexes(backend, table) end
    }
  end

  defp rank(nil), do: 0
  defp rank(value) when is_boolean(value), do: 1
  defp rank(value) when is_integer(value) or is_float(value), do: 2
  defp rank(value) when is_binary(value), do: 3
  defp rank(%Blob{}), do: 4

  defp compare_same_rank(nil, nil), do: 0
  defp compare_same_rank(left, right) when is_boolean(left), do: bool_int(left) - bool_int(right)
  defp compare_same_rank(left, right) when is_number(left), do: number_compare(left, right)
  defp compare_same_rank(left, right) when is_binary(left), do: basic_compare(left, right)
  defp compare_same_rank(%Blob{bytes: left}, %Blob{bytes: right}), do: basic_compare(left, right)

  defp bool_int(true), do: 1
  defp bool_int(false), do: 0

  defp number_compare(left, right) do
    cond do
      left < right -> -1
      left > right -> 1
      true -> 0
    end
  end

  defp basic_compare(left, right) do
    cond do
      left < right -> -1
      left > right -> 1
      true -> 0
    end
  end
end

defmodule CodingAdventures.SqlBackend.ListRowIterator do
  defstruct rows: [], index: 0, closed: false

  def new(rows), do: %__MODULE__{rows: Enum.map(rows, &Map.new/1)}

  def next(%__MODULE__{closed: true} = iterator), do: {nil, iterator}

  def next(%__MODULE__{rows: rows, index: index} = iterator) when index >= length(rows),
    do: {nil, iterator}

  def next(%__MODULE__{rows: rows, index: index} = iterator) do
    {Map.new(Enum.at(rows, index)), %{iterator | index: index + 1}}
  end

  def close(iterator), do: %{iterator | closed: true}

  def to_list(iterator), do: to_list(iterator, [])

  defp to_list(iterator, acc) do
    case next(iterator) do
      {nil, _iterator} -> Enum.reverse(acc)
      {row, iterator} -> to_list(iterator, [row | acc])
    end
  end
end

defmodule CodingAdventures.SqlBackend.Cursor do
  defstruct table_key: nil, rowids: [], index: -1, current_rowid: nil, rows: []

  def new(table_key, rows) do
    %__MODULE__{
      table_key: table_key,
      rowids: Enum.map(rows, & &1.rowid),
      rows: Enum.map(rows, &{&1.rowid, Map.new(&1.row)})
    }
  end

  def next(%__MODULE__{index: index, rows: rows} = cursor) do
    index = index + 1

    case Enum.at(rows, index) do
      nil -> {nil, %{cursor | index: index, current_rowid: nil}}
      {rowid, row} -> {Map.new(row), %{cursor | index: index, current_rowid: rowid}}
    end
  end

  def current_row(%__MODULE__{current_rowid: nil}), do: nil

  def current_row(%__MODULE__{current_rowid: rowid, rows: rows}) do
    case Enum.find(rows, fn {id, _row} -> id == rowid end) do
      nil -> nil
      {_rowid, row} -> Map.new(row)
    end
  end
end

defmodule CodingAdventures.SqlBackend.InMemoryBackend do
  alias CodingAdventures.SqlBackend

  alias CodingAdventures.SqlBackend.{
    ColumnAlreadyExists,
    ColumnDef,
    ColumnNotFound,
    ConstraintViolation,
    Cursor,
    IndexAlreadyExists,
    IndexDef,
    IndexNotFound,
    Internal,
    ListRowIterator,
    TableAlreadyExists,
    TableNotFound,
    TriggerAlreadyExists,
    TriggerDef,
    TriggerNotFound,
    Unsupported
  }

  defstruct tables: %{},
            indexes: %{},
            triggers: %{},
            triggers_by_table: %{},
            user_version: 0,
            schema_version: 0,
            transaction_snapshot: nil,
            current_transaction: nil,
            next_transaction: 1,
            savepoints: []

  defmodule StoredRow do
    defstruct [:rowid, :row]
  end

  defmodule TableState do
    defstruct [:name, columns: [], rows: [], next_rowid: 0]
  end

  def new, do: %__MODULE__{}

  def tables(%__MODULE__{tables: tables}), do: tables |> Map.values() |> Enum.map(& &1.name)
  def columns(backend, table), do: table_state!(backend, table).columns

  def scan(backend, table),
    do:
      backend
      |> table_state!(table)
      |> Map.fetch!(:rows)
      |> Enum.map(& &1.row)
      |> ListRowIterator.new()

  def open_cursor(backend, table),
    do: Cursor.new(normalize(table), table_state!(backend, table).rows)

  def insert(backend, table, row) do
    state = table_state!(backend, table)
    candidate = materialize_row(state, row)
    validate_row!(backend, state, candidate)

    put_state(backend, %{
      state
      | rows: state.rows ++ [%StoredRow{rowid: state.next_rowid, row: candidate}],
        next_rowid: state.next_rowid + 1
    })
  end

  def update(backend, table, %Cursor{table_key: key, current_rowid: rowid}, assignments) do
    state = table_state!(backend, table)

    if key != normalize(state.name) or is_nil(rowid),
      do: raise(Internal, "cursor is not positioned on #{state.name}")

    record = Enum.find(state.rows, &(&1.rowid == rowid)) || raise Internal, "cursor row vanished"

    candidate =
      Enum.reduce(assignments, Map.new(record.row), fn {name, value}, acc ->
        column = find_column(state, name) || raise ColumnNotFound, {state.name, name}
        if not SqlBackend.sql_value?(value), do: raise(Internal, "not a SQL value")
        Map.put(acc, column.name, value)
      end)

    validate_row!(backend, state, candidate, rowid)

    rows =
      Enum.map(state.rows, fn stored ->
        if stored.rowid == rowid, do: %{stored | row: candidate}, else: stored
      end)

    put_state(backend, %{state | rows: rows})
  end

  def delete(backend, table, %Cursor{table_key: key, current_rowid: rowid}) do
    state = table_state!(backend, table)

    if key != normalize(state.name) or is_nil(rowid),
      do: raise(Internal, "cursor is not positioned on #{state.name}")

    put_state(backend, %{state | rows: Enum.reject(state.rows, &(&1.rowid == rowid))})
  end

  def create_table(backend, table, columns, opts \\ []) do
    key = normalize(table)

    cond do
      Map.has_key?(backend.tables, key) and Keyword.get(opts, :if_not_exists, false) ->
        backend

      Map.has_key?(backend.tables, key) ->
        raise TableAlreadyExists, table

      true ->
        columns
        |> Enum.map(&normalize(&1.name))
        |> duplicates()
        |> case do
          [] ->
            put_state(%{backend | schema_version: backend.schema_version + 1}, %TableState{
              name: table,
              columns: columns
            })

          [dup | _] ->
            raise ColumnAlreadyExists, {table, dup}
        end
    end
  end

  def drop_table(backend, table, opts \\ []) do
    key = normalize(table)

    cond do
      Map.has_key?(backend.tables, key) ->
        %{
          backend
          | tables: Map.delete(backend.tables, key),
            indexes:
              Map.reject(backend.indexes, fn {_name, index} -> normalize(index.table) == key end),
            triggers:
              Map.reject(backend.triggers, fn {_name, trigger} ->
                normalize(trigger.table) == key
              end),
            triggers_by_table: Map.delete(backend.triggers_by_table, key),
            schema_version: backend.schema_version + 1
        }

      Keyword.get(opts, :if_exists, false) ->
        backend

      true ->
        raise TableNotFound, table
    end
  end

  def add_column(backend, table, %ColumnDef{} = column) do
    state = table_state!(backend, table)
    if find_column(state, column.name), do: raise(ColumnAlreadyExists, {state.name, column.name})

    if ColumnDef.effective_not_null?(column) and not column.has_default and state.rows != [] do
      raise ConstraintViolation,
            {state.name, column.name, "NOT NULL constraint failed: #{state.name}.#{column.name}"}
    end

    rows =
      Enum.map(state.rows, fn stored ->
        %{stored | row: Map.put(stored.row, column.name, column.default)}
      end)

    put_state(%{backend | schema_version: backend.schema_version + 1}, %{
      state
      | columns: state.columns ++ [column],
        rows: rows
    })
  end

  def create_index(backend, %IndexDef{} = index) do
    key = normalize(index.name)
    if Map.has_key?(backend.indexes, key), do: raise(IndexAlreadyExists, index.name)
    state = table_state!(backend, index.table)
    Enum.each(index.columns, fn column -> real_column!(state, column) end)
    if index.unique, do: validate_unique_index!(state, index)

    %{
      backend
      | indexes: Map.put(backend.indexes, key, index),
        schema_version: backend.schema_version + 1
    }
  end

  def drop_index(backend, name, opts \\ []) do
    key = normalize(name)

    cond do
      Map.has_key?(backend.indexes, key) ->
        %{
          backend
          | indexes: Map.delete(backend.indexes, key),
            schema_version: backend.schema_version + 1
        }

      Keyword.get(opts, :if_exists, false) ->
        backend

      true ->
        raise IndexNotFound, name
    end
  end

  def list_indexes(backend, table \\ nil) do
    backend.indexes
    |> Map.values()
    |> Enum.filter(fn index -> is_nil(table) or normalize(index.table) == normalize(table) end)
  end

  def scan_index(backend, index_name, lo \\ nil, hi \\ nil, opts \\ []) do
    index = Map.get(backend.indexes, normalize(index_name)) || raise(IndexNotFound, index_name)
    state = table_state!(backend, index.table)

    state.rows
    |> Enum.map(fn stored -> {index_key(state, stored.row, index.columns), stored.rowid} end)
    |> Enum.sort(fn {left, left_id}, {right, right_id} ->
      compare_keys(left, right) < 0 or (compare_keys(left, right) == 0 and left_id < right_id)
    end)
    |> Enum.filter(fn {key, _rowid} ->
      lower_ok =
        is_nil(lo) or compare_keys(key, lo) > 0 or
          (Keyword.get(opts, :lo_inclusive, true) and compare_keys(key, lo) == 0)

      upper_ok =
        is_nil(hi) or compare_keys(key, hi) < 0 or
          (Keyword.get(opts, :hi_inclusive, true) and compare_keys(key, hi) == 0)

      lower_ok and upper_ok
    end)
    |> Enum.map(&elem(&1, 1))
  end

  def scan_by_rowids(backend, table, rowids) do
    rows_by_id =
      backend |> table_state!(table) |> Map.fetch!(:rows) |> Map.new(&{&1.rowid, &1.row})

    rowids
    |> Enum.flat_map(fn rowid -> if rows_by_id[rowid], do: [rows_by_id[rowid]], else: [] end)
    |> ListRowIterator.new()
  end

  def begin_transaction(%__MODULE__{current_transaction: nil} = backend) do
    handle = backend.next_transaction

    {%{
       backend
       | transaction_snapshot: snapshot(backend),
         current_transaction: handle,
         next_transaction: handle + 1
     }, handle}
  end

  def begin_transaction(_backend), do: raise(Unsupported, "nested transactions")

  def commit(%__MODULE__{current_transaction: handle} = backend, handle),
    do: %{backend | transaction_snapshot: nil, current_transaction: nil, savepoints: []}

  def commit(_backend, _handle), do: raise(Internal, "invalid transaction handle")

  def rollback(
        %__MODULE__{current_transaction: handle, transaction_snapshot: snapshot} = backend,
        handle
      ) do
    restored =
      snapshot
      |> Map.put(:next_transaction, backend.next_transaction)
      |> Map.put(:current_transaction, nil)
      |> Map.put(:transaction_snapshot, nil)

    struct(__MODULE__, restored)
  end

  def rollback(_backend, _handle), do: raise(Internal, "invalid transaction handle")

  def create_savepoint(%__MODULE__{current_transaction: nil}, _name),
    do: raise(Unsupported, "savepoints outside transaction")

  def create_savepoint(backend, name),
    do: %{backend | savepoints: backend.savepoints ++ [{name, snapshot(backend)}]}

  def release_savepoint(backend, name) do
    index = savepoint_index!(backend, name)
    %{backend | savepoints: Enum.take(backend.savepoints, index)}
  end

  def rollback_to_savepoint(backend, name) do
    index = savepoint_index!(backend, name)
    {_name, snapshot} = Enum.at(backend.savepoints, index)

    restored =
      snapshot
      |> Map.put(:savepoints, Enum.take(backend.savepoints, index + 1))
      |> Map.put(:next_transaction, backend.next_transaction)

    struct(__MODULE__, restored)
  end

  def create_trigger(backend, %TriggerDef{} = trigger) do
    key = normalize(trigger.name)
    if Map.has_key?(backend.triggers, key), do: raise(TriggerAlreadyExists, trigger.name)
    table_state!(backend, trigger.table)

    trigger = %{
      trigger
      | timing: String.upcase(trigger.timing),
        event: String.upcase(trigger.event)
    }

    table_key = normalize(trigger.table)

    %{
      backend
      | triggers: Map.put(backend.triggers, key, trigger),
        triggers_by_table:
          Map.update(backend.triggers_by_table, table_key, [key], &(&1 ++ [key])),
        schema_version: backend.schema_version + 1
    }
  end

  def drop_trigger(backend, name, opts \\ []) do
    key = normalize(name)

    case Map.fetch(backend.triggers, key) do
      {:ok, trigger} ->
        table_key = normalize(trigger.table)

        %{
          backend
          | triggers: Map.delete(backend.triggers, key),
            triggers_by_table:
              Map.update(
                backend.triggers_by_table,
                table_key,
                [],
                &Enum.reject(&1, fn item -> item == key end)
              ),
            schema_version: backend.schema_version + 1
        }

      :error ->
        if Keyword.get(opts, :if_exists, false), do: backend, else: raise(TriggerNotFound, name)
    end
  end

  def list_triggers(backend, table) do
    backend.triggers_by_table
    |> Map.get(normalize(table), [])
    |> Enum.flat_map(fn key ->
      if backend.triggers[key], do: [backend.triggers[key]], else: []
    end)
  end

  defp table_state!(backend, table),
    do: Map.get(backend.tables, normalize(table)) || raise(TableNotFound, table)

  defp put_state(backend, state),
    do: %{backend | tables: Map.put(backend.tables, normalize(state.name), state)}

  defp materialize_row(state, row) do
    state.columns
    |> Enum.reduce(%{}, fn column, acc ->
      {present?, value} = find_value(row, column.name)

      value =
        cond do
          present? -> value
          column.autoincrement and column.primary_key -> next_autoincrement_value(state, column)
          column.has_default -> column.default
          true -> nil
        end

      if not SqlBackend.sql_value?(value), do: raise(Internal, "not a SQL value")
      Map.put(acc, column.name, value)
    end)
    |> tap(fn _candidate ->
      Enum.each(Map.keys(row), fn name ->
        if is_nil(find_column(state, name)), do: raise(ColumnNotFound, {state.name, name})
      end)
    end)
  end

  defp find_value(row, name) do
    case Enum.find(row, fn {key, _value} -> normalize(key) == normalize(name) end) do
      nil -> {false, nil}
      {_key, value} -> {true, value}
    end
  end

  defp next_autoincrement_value(state, column) do
    state.rows
    |> Enum.map(&Map.get(&1.row, column.name))
    |> Enum.filter(&is_integer/1)
    |> Enum.max(fn -> 0 end)
    |> Kernel.+(1)
  end

  defp validate_row!(backend, state, row, skip_rowid \\ nil) do
    Enum.each(state.columns, fn column ->
      value = row[column.name]

      if ColumnDef.effective_not_null?(column) and is_nil(value) do
        raise ConstraintViolation,
              {state.name, column.name,
               "NOT NULL constraint failed: #{state.name}.#{column.name}"}
      end

      if ColumnDef.effective_unique?(column) and not is_nil(value) do
        Enum.each(state.rows, fn stored ->
          if stored.rowid != skip_rowid and
               SqlBackend.compare(stored.row[column.name], value) == 0 do
            label = if column.primary_key, do: "PRIMARY KEY", else: "UNIQUE"

            raise ConstraintViolation,
                  {state.name, column.name,
                   "#{label} constraint failed: #{state.name}.#{column.name}"}
          end
        end)
      end
    end)

    backend.indexes
    |> Map.values()
    |> Enum.filter(&(&1.unique and normalize(&1.table) == normalize(state.name)))
    |> Enum.each(&validate_unique_index!(state, &1, row, skip_rowid))
  end

  defp validate_unique_index!(state, index, candidate \\ nil, skip_rowid \\ nil) do
    if candidate do
      candidate_key = index_key(state, candidate, index.columns)

      if Enum.any?(candidate_key, &is_nil/1),
        do: :ok,
        else:
          Enum.each(state.rows, fn stored ->
            if stored.rowid != skip_rowid and
                 compare_keys(index_key(state, stored.row, index.columns), candidate_key) == 0 do
              raise ConstraintViolation,
                    {state.name, Enum.join(index.columns, ","),
                     "UNIQUE constraint failed: #{state.name}.#{Enum.join(index.columns, ",")}"}
            end
          end)
    else
      state.rows
      |> Enum.map(&index_key(state, &1.row, index.columns))
      |> Enum.reject(&Enum.any?(&1, fn value -> is_nil(value) end))
      |> then(fn keys ->
        if length(keys) != length(Enum.uniq(keys)),
          do:
            raise(
              ConstraintViolation,
              {state.name, Enum.join(index.columns, ","),
               "UNIQUE constraint failed: #{state.name}.#{Enum.join(index.columns, ",")}"}
            )
      end)
    end
  end

  defp find_column(state, name),
    do: Enum.find(state.columns, &(normalize(&1.name) == normalize(name)))

  defp real_column!(state, name),
    do: (find_column(state, name) || raise(ColumnNotFound, {state.name, name})).name

  defp index_key(state, row, columns),
    do: Enum.map(columns, &Map.get(row, real_column!(state, &1)))

  defp compare_keys(left, right) do
    left
    |> Enum.zip(right)
    |> Enum.reduce_while(0, fn {l, r}, _acc ->
      case SqlBackend.compare(l, r) do
        0 -> {:cont, 0}
        cmp -> {:halt, cmp}
      end
    end)
  end

  defp duplicates(values), do: values -- Enum.uniq(values)
  defp normalize(value), do: value |> to_string() |> String.downcase()

  defp snapshot(backend),
    do:
      Map.take(backend, [
        :tables,
        :indexes,
        :triggers,
        :triggers_by_table,
        :user_version,
        :schema_version,
        :transaction_snapshot,
        :current_transaction,
        :next_transaction,
        :savepoints
      ])

  defp savepoint_index!(%__MODULE__{current_transaction: nil}, _name),
    do: raise(Unsupported, "savepoints outside transaction")

  defp savepoint_index!(backend, name) do
    index =
      backend.savepoints |> Enum.find_index(fn {candidate, _snapshot} -> candidate == name end)

    if is_nil(index), do: raise(Internal, "savepoint not found: #{name}"), else: index
  end
end
