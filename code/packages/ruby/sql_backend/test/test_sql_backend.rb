# frozen_string_literal: true

require_relative "test_helper"

SB = CodingAdventures::SqlBackend

class SqlBackendTest < Minitest::Test
  def test_sql_value_helpers_classify_and_compare_values
    assert SB::SqlValues.sql_value?(nil)
    assert SB::SqlValues.sql_value?(true)
    assert SB::SqlValues.sql_value?(42)
    assert SB::SqlValues.sql_value?(1.5)
    assert SB::SqlValues.sql_value?("text")
    assert SB::SqlValues.sql_value?(SB::Blob.new([1, 2, 3]))
    refute SB::SqlValues.sql_value?(Float::NAN)
    refute SB::SqlValues.sql_value?(Object.new)

    assert_equal "NULL", SB::SqlValues.type_name(nil)
    assert_equal "BOOLEAN", SB::SqlValues.type_name(false)
    assert_equal "INTEGER", SB::SqlValues.type_name(1)
    assert_equal "REAL", SB::SqlValues.type_name(1.0)
    assert_equal "TEXT", SB::SqlValues.type_name("x")
    assert_equal "BLOB", SB::SqlValues.type_name(SB::Blob.new("x"))

    assert_operator SB::SqlValues.compare(nil, 1), :<, 0
    assert_operator SB::SqlValues.compare(false, true), :<, 0
    assert_operator SB::SqlValues.compare(1, 2), :<, 0
    assert_operator SB::SqlValues.compare("b", "a"), :>, 0
    assert_equal 0, SB::SqlValues.compare(SB::Blob.new([1]), SB::Blob.new([1]))
  end

  def test_list_iterators_and_cursors_return_row_copies
    rows = [{"id" => 1, "name" => "Ada"}, {"id" => 2, "name" => "Grace"}]
    iterator = SB::ListRowIterator.new(rows)
    first = iterator.next
    first["name"] = "mutated"

    assert_equal({"id" => 2, "name" => "Grace"}, iterator.next)
    assert_nil iterator.next

    cursor = SB::ListCursor.new(rows, table_key: "users")
    assert_equal({"id" => 1, "name" => "Ada"}, cursor.next)
    assert_equal 0, cursor.current_index
    current = cursor.current_row
    current["name"] = "mutated"
    assert_equal({"id" => 1, "name" => "Ada"}, cursor.current_row)
    assert_equal "users", cursor.table_key
  end

  def test_create_insert_scan_and_schema_provider
    backend = new_users_backend
    backend.insert("users", {"id" => 1, "name" => "Ada"})
    backend.insert("users", {"id" => 2, "name" => "Grace", "email" => "grace@example.test"})

    assert_equal ["users"], backend.tables
    assert_equal ["id", "name", "email"], backend.columns("users").map(&:name)
    assert_equal ["id", "name", "email"], SB.backend_as_schema_provider(backend).columns("users")
    assert_equal [
      {"id" => 1, "name" => "Ada", "email" => nil},
      {"id" => 2, "name" => "Grace", "email" => "grace@example.test"}
    ], backend.scan("USERS").to_a
  end

  def test_constraints_and_unknown_columns
    backend = new_users_backend
    backend.insert("users", {"id" => 1, "name" => "Ada"})

    assert_raises(SB::ConstraintViolation) do
      backend.insert("users", {"id" => 2})
    end
    duplicate = assert_raises(SB::ConstraintViolation) do
      backend.insert("users", {"id" => 1, "name" => "Ada Again"})
    end
    assert_match(/PRIMARY KEY constraint failed/, duplicate.message)
    assert_raises(SB::ColumnNotFound) do
      backend.insert("users", {"id" => 3, "name" => "Lin", "missing" => 1})
    end
  end

  def test_unique_columns_allow_multiple_nulls_but_reject_duplicate_values
    backend = new_users_backend
    backend.insert("users", {"id" => 1, "name" => "Ada"})
    backend.insert("users", {"id" => 2, "name" => "Grace"})
    backend.insert("users", {"id" => 3, "name" => "Lin", "email" => "lin@example.test"})

    error = assert_raises(SB::ConstraintViolation) do
      backend.insert("users", {"id" => 4, "name" => "Other Lin", "email" => "lin@example.test"})
    end
    assert_match(/UNIQUE constraint failed/, error.message)
  end

  def test_update_and_delete_use_positioned_cursors
    backend = new_users_backend
    backend.insert("users", {"id" => 1, "name" => "Ada"})
    backend.insert("users", {"id" => 2, "name" => "Grace"})

    cursor = backend.open_cursor("users")
    cursor.next
    backend.update("users", cursor, {"name" => "Augusta Ada"})
    assert_equal "Augusta Ada", backend.scan("users").to_a.first["name"]

    cursor.next
    backend.delete("users", cursor)
    assert_equal [{"id" => 1, "name" => "Augusta Ada", "email" => nil}], backend.scan("users").to_a
    fresh_cursor = backend.open_cursor("users")
    fresh_cursor.next
    assert_raises(SB::ColumnNotFound) { backend.update("users", fresh_cursor, {"missing" => 1}) }
  end

  def test_table_ddl_and_add_column_defaults
    backend = new_users_backend
    assert_raises(SB::TableAlreadyExists) do
      backend.create_table("users", [], if_not_exists: false)
    end
    backend.create_table("users", [], if_not_exists: true)
    backend.insert("users", {"id" => 1, "name" => "Ada"})

    backend.add_column("users", SB::ColumnDef.new(name: "active", type_name: "BOOLEAN", default: true))
    assert_equal true, backend.scan("users").to_a.first["active"]
    assert_raises(SB::ColumnAlreadyExists) do
      backend.add_column("users", SB::ColumnDef.new(name: "ACTIVE", type_name: "BOOLEAN"))
    end

    backend.drop_table("users", if_exists: false)
    assert_raises(SB::TableNotFound) { backend.columns("users") }
    backend.drop_table("users", if_exists: true)
  end

  def test_indexes_scan_rowids_and_drop
    backend = new_users_backend
    backend.insert("users", {"id" => 1, "name" => "Ada"})
    backend.insert("users", {"id" => 2, "name" => "Grace"})
    backend.insert("users", {"id" => 3, "name" => "Lin"})
    backend.create_index(SB::IndexDef.new(name: "idx_users_name", table: "users", columns: ["name"]))

    rowids = backend.scan_index("idx_users_name", ["G"], ["M"], lo_inclusive: false, hi_inclusive: false)
    assert_equal [{"id" => 2, "name" => "Grace", "email" => nil}, {"id" => 3, "name" => "Lin", "email" => nil}],
      backend.scan_by_rowids("users", rowids).to_a
    assert_equal ["idx_users_name"], backend.list_indexes("users").map(&:name)
    assert_raises(SB::IndexAlreadyExists) do
      backend.create_index(SB::IndexDef.new(name: "idx_users_name", table: "users", columns: ["id"]))
    end
    backend.drop_index("idx_users_name")
    assert_equal [], backend.list_indexes
    backend.drop_index("idx_users_name", if_exists: true)
    assert_raises(SB::IndexNotFound) { backend.scan_index("missing", nil, nil) }
  end

  def test_unique_indexes_are_enforced_on_insert_and_update
    backend = new_users_backend
    backend.create_index(SB::IndexDef.new(name: "idx_email", table: "users", columns: ["email"], unique: true))
    backend.insert("users", {"id" => 1, "name" => "Ada", "email" => "ada@example.test"})
    backend.insert("users", {"id" => 2, "name" => "Grace", "email" => "grace@example.test"})

    assert_raises(SB::ConstraintViolation) do
      backend.insert("users", {"id" => 3, "name" => "Other Ada", "email" => "ada@example.test"})
    end
    cursor = backend.open_cursor("users")
    cursor.next
    cursor.next
    assert_raises(SB::ConstraintViolation) do
      backend.update("users", cursor, {"email" => "ada@example.test"})
    end
  end

  def test_transactions_commit_and_rollback
    backend = new_users_backend
    handle = backend.begin_transaction
    backend.insert("users", {"id" => 1, "name" => "Ada"})
    assert_equal handle, backend.current_transaction
    backend.rollback(handle)
    assert_equal [], backend.scan("users").to_a

    handle = backend.begin_transaction
    backend.insert("users", {"id" => 2, "name" => "Grace"})
    backend.commit(handle)
    assert_nil backend.current_transaction
    assert_equal [{"id" => 2, "name" => "Grace", "email" => nil}], backend.scan("users").to_a
  end

  def test_savepoints_keep_the_named_point_alive_after_rollback
    backend = new_users_backend
    handle = backend.begin_transaction
    backend.insert("users", {"id" => 1, "name" => "Ada"})
    backend.create_savepoint("after_ada")
    backend.insert("users", {"id" => 2, "name" => "Grace"})
    backend.rollback_to_savepoint("after_ada")
    assert_equal [{"id" => 1, "name" => "Ada", "email" => nil}], backend.scan("users").to_a
    backend.release_savepoint("after_ada")
    backend.commit(handle)
  end

  def test_triggers_and_version_fields
    backend = new_users_backend
    initial_schema_version = backend.schema_version
    trigger = SB::TriggerDef.new(
      name: "users_ai",
      table: "users",
      timing: "after",
      event: "insert",
      body: "SELECT 1"
    )
    backend.create_trigger(trigger)

    assert_operator backend.schema_version, :>, initial_schema_version
    assert_equal [trigger], backend.list_triggers("users")
    assert_raises(SB::TriggerAlreadyExists) { backend.create_trigger(trigger) }

    backend.user_version = 7
    assert_equal 7, backend.user_version
    backend.drop_trigger("users_ai")
    assert_equal [], backend.list_triggers("users")
    backend.drop_trigger("users_ai", if_exists: true)
    assert_raises(SB::TriggerNotFound) { backend.drop_trigger("users_ai") }
  end

  def test_backend_errors_compare_by_payload
    assert_equal SB::TableNotFound.new("users"), SB::TableNotFound.new(table: "users")
    assert_equal SB::ColumnNotFound.new("users", "id"), SB::ColumnNotFound.new(table: "users", column: "id")
    assert_equal SB::Unsupported.new("writes"), SB::Unsupported.new(operation: "writes")
    refute_equal SB::IndexNotFound.new("a"), SB::IndexNotFound.new("b")
  end

  private

  def new_users_backend
    backend = SB::InMemoryBackend.new
    backend.create_table(
      "users",
      [
        SB::ColumnDef.new(name: "id", type_name: "INTEGER", primary_key: true),
        SB::ColumnDef.new(name: "name", type_name: "TEXT", not_null: true),
        SB::ColumnDef.new(name: "email", type_name: "TEXT", unique: true)
      ],
      if_not_exists: false
    )
    backend
  end
end
