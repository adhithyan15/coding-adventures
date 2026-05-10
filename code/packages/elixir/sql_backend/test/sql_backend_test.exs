defmodule CodingAdventures.SqlBackendTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.SqlBackend
  alias CodingAdventures.SqlBackend.Cursor
  alias CodingAdventures.SqlBackend.InMemoryBackend
  alias CodingAdventures.SqlBackend.ListRowIterator

  alias CodingAdventures.SqlBackend.{
    ColumnAlreadyExists,
    ColumnDef,
    ColumnNotFound,
    ConstraintViolation,
    IndexAlreadyExists,
    IndexDef,
    IndexNotFound,
    TableAlreadyExists,
    TableNotFound,
    TriggerAlreadyExists,
    TriggerDef,
    TriggerNotFound
  }

  test "classifies and compares SQL values" do
    assert SqlBackend.sql_value?(nil)
    assert SqlBackend.sql_value?(true)
    assert SqlBackend.sql_value?(42)
    assert SqlBackend.sql_value?(1.5)
    assert SqlBackend.sql_value?("text")
    assert SqlBackend.sql_value?(SqlBackend.blob("abc"))
    refute SqlBackend.sql_value?([])

    assert SqlBackend.type_name(nil) == "NULL"
    assert SqlBackend.type_name(false) == "BOOLEAN"
    assert SqlBackend.type_name(1) == "INTEGER"
    assert SqlBackend.type_name(1.0) == "REAL"
    assert SqlBackend.type_name("x") == "TEXT"
    assert SqlBackend.type_name(SqlBackend.blob("x")) == "BLOB"

    assert SqlBackend.compare(nil, 1) < 0
    assert SqlBackend.compare(false, true) < 0
    assert SqlBackend.compare(1, 2) < 0
    assert SqlBackend.compare("b", "a") > 0
    assert SqlBackend.compare(SqlBackend.blob("a"), SqlBackend.blob("a")) == 0
  end

  test "iterators and cursors expose positioned copies" do
    iterator = ListRowIterator.new([row("id", 1, "name", "Ada"), row("id", 2, "name", "Grace")])
    {first, iterator} = ListRowIterator.next(iterator)
    assert first["name"] == "Ada"

    first = Map.put(first, "name", "mutated")
    assert first["name"] == "mutated"

    {second, iterator} = ListRowIterator.next(iterator)
    assert second["name"] == "Grace"
    assert {nil, ^iterator} = ListRowIterator.next(iterator)

    cursor = InMemoryBackend.open_cursor(users(), "users")
    {first_row, cursor} = Cursor.next(cursor)
    assert first_row["name"] == "Ada"

    current = Cursor.current_row(cursor)
    assert current["name"] == "Ada"
    assert Map.put(current, "name", "mutated")["name"] == "mutated"
    assert Cursor.current_row(cursor)["name"] == "Ada"
  end

  test "creates tables, inserts rows, scans rows, and adapts schema" do
    backend = users()
    assert InMemoryBackend.tables(backend) == ["users"]

    assert InMemoryBackend.columns(backend, "USERS") |> Enum.map(& &1.name) == [
             "id",
             "name",
             "email"
           ]

    provider = SqlBackend.backend_as_schema_provider(backend)
    assert provider[:columns].("users") == ["id", "name", "email"]

    rows = InMemoryBackend.scan(backend, "users") |> ListRowIterator.to_list()
    assert length(rows) == 2
    assert hd(rows)["name"] == "Ada"
    assert Enum.at(rows, 1)["email"] == nil
  end

  test "rejects bad rows with typed constraint errors" do
    backend = users()

    assert_raise ConstraintViolation, fn ->
      InMemoryBackend.insert(backend, "users", row("id", 2))
    end

    assert_raise ConstraintViolation, fn ->
      InMemoryBackend.insert(backend, "users", row("id", 1, "name", "Ada Again"))
    end

    assert_raise ColumnNotFound, fn ->
      InMemoryBackend.insert(backend, "users", row("id", 3, "name", "Lin", "missing", 1))
    end

    backend =
      InMemoryBackend.insert(
        backend,
        "users",
        row("id", 3, "name", "Lin", "email", "lin@example.test")
      )

    assert_raise ConstraintViolation, fn ->
      InMemoryBackend.insert(
        backend,
        "users",
        row("id", 4, "name", "Other Lin", "email", "lin@example.test")
      )
    end
  end

  test "updates and deletes positioned rows" do
    backend = users()
    {_row, cursor} = backend |> InMemoryBackend.open_cursor("users") |> Cursor.next()

    backend = InMemoryBackend.update(backend, "users", cursor, %{"name" => "Augusta Ada"})

    assert backend
           |> InMemoryBackend.scan("users")
           |> ListRowIterator.to_list()
           |> hd()
           |> Map.fetch!("name") == "Augusta Ada"

    {_row, cursor} = Cursor.next(cursor)
    backend = InMemoryBackend.delete(backend, "users", cursor)
    rows = InMemoryBackend.scan(backend, "users") |> ListRowIterator.to_list()

    assert length(rows) == 1
    assert hd(rows)["name"] == "Augusta Ada"
  end

  test "creates, alters, and drops tables" do
    backend = users()

    assert_raise TableAlreadyExists, fn ->
      InMemoryBackend.create_table(backend, "users", [], if_not_exists: false)
    end

    backend = InMemoryBackend.create_table(backend, "users", [], if_not_exists: true)

    backend =
      InMemoryBackend.add_column(
        backend,
        "users",
        SqlBackend.column_def(name: "active", type_name: "BOOLEAN", default: true)
      )

    assert backend
           |> InMemoryBackend.scan("users")
           |> ListRowIterator.to_list()
           |> hd()
           |> Map.fetch!("active") == true

    assert_raise ColumnAlreadyExists, fn ->
      InMemoryBackend.add_column(backend, "users", %ColumnDef{
        name: "ACTIVE",
        type_name: "BOOLEAN"
      })
    end

    backend = InMemoryBackend.drop_table(backend, "users", if_exists: false)
    assert_raise TableNotFound, fn -> InMemoryBackend.columns(backend, "users") end
    assert InMemoryBackend.drop_table(backend, "users", if_exists: true) == backend
  end

  test "scans indexes, fetches rowids, and enforces unique indexes" do
    backend =
      users()
      |> InMemoryBackend.insert("users", row("id", 3, "name", "Lin"))
      |> InMemoryBackend.create_index(%IndexDef{
        name: "idx_users_name",
        table: "users",
        columns: ["name"]
      })

    rowids =
      InMemoryBackend.scan_index(backend, "idx_users_name", ["G"], ["M"],
        lo_inclusive: false,
        hi_inclusive: false
      )

    rows = InMemoryBackend.scan_by_rowids(backend, "users", rowids) |> ListRowIterator.to_list()
    assert Enum.map(rows, & &1["name"]) == ["Grace", "Lin"]
    assert hd(InMemoryBackend.list_indexes(backend, "users")).name == "idx_users_name"

    assert_raise IndexAlreadyExists, fn ->
      InMemoryBackend.create_index(backend, %IndexDef{
        name: "idx_users_name",
        table: "users",
        columns: ["id"]
      })
    end

    backend = InMemoryBackend.drop_index(backend, "idx_users_name")
    assert InMemoryBackend.list_indexes(backend) == []
    assert InMemoryBackend.drop_index(backend, "idx_users_name", if_exists: true) == backend
    assert_raise IndexNotFound, fn -> InMemoryBackend.scan_index(backend, "missing") end

    backend =
      InMemoryBackend.create_index(backend, %IndexDef{
        name: "idx_name_unique",
        table: "users",
        columns: ["name"],
        unique: true
      })

    assert_raise ConstraintViolation, fn ->
      InMemoryBackend.insert(backend, "users", row("id", 4, "name", "Lin"))
    end
  end

  test "transactions and savepoints restore snapshots" do
    {backend, handle} = users() |> InMemoryBackend.begin_transaction()
    backend = InMemoryBackend.insert(backend, "users", row("id", 3, "name", "Lin"))
    assert backend.current_transaction == handle

    backend = InMemoryBackend.rollback(backend, handle)
    assert backend |> InMemoryBackend.scan("users") |> ListRowIterator.to_list() |> length() == 2

    {backend, handle} = InMemoryBackend.begin_transaction(backend)

    backend =
      backend
      |> InMemoryBackend.insert("users", row("id", 3, "name", "Lin"))
      |> InMemoryBackend.create_savepoint("after_lin")
      |> InMemoryBackend.insert("users", row("id", 4, "name", "Katherine"))
      |> InMemoryBackend.rollback_to_savepoint("after_lin")

    assert backend |> InMemoryBackend.scan("users") |> ListRowIterator.to_list() |> length() == 3

    backend = InMemoryBackend.rollback(backend, handle)
    assert backend |> InMemoryBackend.scan("users") |> ListRowIterator.to_list() |> length() == 2

    {backend, handle} = InMemoryBackend.begin_transaction(backend)

    backend =
      backend
      |> InMemoryBackend.insert("users", row("id", 3, "name", "Lin"))
      |> InMemoryBackend.create_savepoint("after_lin")
      |> InMemoryBackend.release_savepoint("after_lin")
      |> InMemoryBackend.commit(handle)

    assert backend.current_transaction == nil
    assert backend |> InMemoryBackend.scan("users") |> ListRowIterator.to_list() |> length() == 3
  end

  test "stores triggers and version fields" do
    backend = users()
    initial = backend.schema_version

    trigger = %TriggerDef{
      name: "users_ai",
      table: "users",
      timing: "after",
      event: "insert",
      body: "SELECT 1"
    }

    backend = InMemoryBackend.create_trigger(backend, trigger)

    assert backend.schema_version > initial
    assert hd(InMemoryBackend.list_triggers(backend, "users")).name == "users_ai"
    assert hd(InMemoryBackend.list_triggers(backend, "users")).timing == "AFTER"

    assert_raise TriggerAlreadyExists, fn -> InMemoryBackend.create_trigger(backend, trigger) end

    backend = %{backend | user_version: 7}
    assert backend.user_version == 7

    backend = InMemoryBackend.drop_trigger(backend, "users_ai")
    assert InMemoryBackend.list_triggers(backend, "users") == []
    assert InMemoryBackend.drop_trigger(backend, "users_ai", if_exists: true) == backend
    assert_raise TriggerNotFound, fn -> InMemoryBackend.drop_trigger(backend, "users_ai") end
  end

  defp users do
    InMemoryBackend.new()
    |> InMemoryBackend.create_table(
      "users",
      [
        SqlBackend.column_def(name: "id", type_name: "INTEGER", primary_key: true),
        SqlBackend.column_def(name: "name", type_name: "TEXT", not_null: true),
        SqlBackend.column_def(name: "email", type_name: "TEXT", unique: true)
      ]
    )
    |> InMemoryBackend.insert("users", row("id", 1, "name", "Ada", "email", "ada@example.test"))
    |> InMemoryBackend.insert("users", row("id", 2, "name", "Grace"))
  end

  defp row(key, value), do: %{key => value}
  defp row(key1, value1, key2, value2), do: %{key1 => value1, key2 => value2}

  defp row(key1, value1, key2, value2, key3, value3),
    do: %{key1 => value1, key2 => value2, key3 => value3}
end
