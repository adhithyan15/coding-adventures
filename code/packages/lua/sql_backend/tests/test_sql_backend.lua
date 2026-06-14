local backend = require("coding_adventures.sql_backend")

local function new_users_backend()
  local db = backend.InMemoryBackend.new()
  db:create_table("users", {
    backend.column_def({ name = "id", type_name = "INTEGER", primary_key = true }),
    backend.column_def({ name = "name", type_name = "TEXT", not_null = true }),
    backend.column_def({ name = "email", type_name = "TEXT", unique = true }),
  }, { if_not_exists = false })
  return db
end

local function expect_error(kind, fn)
  local ok, err = pcall(fn)
  assert.is_false(ok)
  assert.equals(kind, err.kind)
  return err
end

describe("sql_backend values", function()
  it("classifies and compares SQL values", function()
    assert.is_true(backend.is_sql_value(nil))
    assert.is_true(backend.is_sql_value(false))
    assert.is_true(backend.is_sql_value(42))
    assert.is_true(backend.is_sql_value(1.5))
    assert.is_true(backend.is_sql_value("text"))
    assert.is_true(backend.is_sql_value(backend.blob("abc")))
    assert.is_false(backend.is_sql_value({}))

    assert.equals("NULL", backend.sql_type_name(nil))
    assert.equals("BOOLEAN", backend.sql_type_name(true))
    assert.equals("TEXT", backend.sql_type_name("x"))
    assert.equals("BLOB", backend.sql_type_name(backend.blob("x")))

    assert.is_true(backend.compare(nil, 1) < 0)
    assert.is_true(backend.compare(false, true) < 0)
    assert.is_true(backend.compare(1, 2) < 0)
    assert.is_true(backend.compare("b", "a") > 0)
    assert.equals(0, backend.compare(backend.blob("a"), backend.blob("a")))
  end)
end)

describe("row iterators and cursors", function()
  it("return copies rather than exposing stored rows", function()
    local rows = { { id = 1, name = "Ada" }, { id = 2, name = "Grace" } }
    local iter = backend.ListRowIterator.new(rows)
    local first = iter:next()
    first.name = "mutated"
    assert.equals("Grace", iter:next().name)
    assert.is_nil(iter:next())

    local cursor = backend.ListCursor.new(rows, "users")
    assert.equals("Ada", cursor:next().name)
    local current = cursor:current_row()
    current.name = "mutated"
    assert.equals("Ada", cursor:current_row().name)
    assert.equals(1, cursor:current_index())
  end)
end)

describe("in-memory backend schema and scans", function()
  it("creates tables, inserts rows, and exposes schema provider columns", function()
    local db = new_users_backend()
    db:insert("users", { id = 1, name = "Ada" })
    db:insert("users", { id = 2, name = "Grace", email = "grace@example.test" })

    assert.equals("users", db:tables()[1])
    assert.equals("id", db:columns("USERS")[1].name)
    assert.equals("name", backend.backend_as_schema_provider(db):columns("users")[2])

    local rows = db:scan("users"):to_table()
    assert.equals(2, #rows)
    assert.equals("Ada", rows[1].name)
    assert.is_nil(rows[1].email)
    assert.equals("grace@example.test", rows[2].email)
  end)

  it("raises typed errors for duplicate tables and unknown tables", function()
    local db = new_users_backend()
    expect_error("TableAlreadyExists", function()
      db:create_table("users", {}, { if_not_exists = false })
    end)
    db:create_table("users", {}, { if_not_exists = true })
    expect_error("TableNotFound", function() db:columns("missing") end)
  end)
end)

describe("constraints", function()
  it("enforces primary key, not null, unique, and unknown column checks", function()
    local db = new_users_backend()
    db:insert("users", { id = 1, name = "Ada" })

    expect_error("ConstraintViolation", function() db:insert("users", { id = 2 }) end)
    expect_error("ConstraintViolation", function() db:insert("users", { id = 1, name = "Ada Again" }) end)
    expect_error("ColumnNotFound", function() db:insert("users", { id = 3, name = "Lin", missing = 1 }) end)
  end)

  it("allows multiple NULL unique values and rejects duplicate non-NULL values", function()
    local db = new_users_backend()
    db:insert("users", { id = 1, name = "Ada" })
    db:insert("users", { id = 2, name = "Grace" })
    db:insert("users", { id = 3, name = "Lin", email = "lin@example.test" })

    expect_error("ConstraintViolation", function()
      db:insert("users", { id = 4, name = "Other Lin", email = "lin@example.test" })
    end)
  end)
end)

describe("positioned writes", function()
  it("updates and deletes the current cursor row", function()
    local db = new_users_backend()
    db:insert("users", { id = 1, name = "Ada" })
    db:insert("users", { id = 2, name = "Grace" })

    local cursor = db:open_cursor("users")
    cursor:next()
    db:update("users", cursor, { name = "Augusta Ada" })
    assert.equals("Augusta Ada", db:scan("users"):to_table()[1].name)

    cursor:next()
    db:delete("users", cursor)
    local rows = db:scan("users"):to_table()
    assert.equals(1, #rows)
    assert.equals("Augusta Ada", rows[1].name)
  end)
end)

describe("DDL", function()
  it("adds columns with defaults and drops tables", function()
    local db = new_users_backend()
    db:insert("users", { id = 1, name = "Ada" })
    db:add_column("users", backend.column_def({ name = "active", type_name = "BOOLEAN", default = true }))

    assert.is_true(db:scan("users"):to_table()[1].active)
    expect_error("ColumnAlreadyExists", function()
      db:add_column("users", backend.column_def({ name = "ACTIVE", type_name = "BOOLEAN" }))
    end)

    db:drop_table("users", { if_exists = false })
    expect_error("TableNotFound", function() db:scan("users") end)
    db:drop_table("users", { if_exists = true })
  end)
end)

describe("indexes", function()
  it("scans ranges and fetches rows by rowid", function()
    local db = new_users_backend()
    db:insert("users", { id = 1, name = "Ada" })
    db:insert("users", { id = 2, name = "Grace" })
    db:insert("users", { id = 3, name = "Lin" })
    db:create_index(backend.index_def({ name = "idx_users_name", table = "users", columns = { "name" } }))

    local rowids = db:scan_index("idx_users_name", { "G" }, { "M" }, { lo_inclusive = false, hi_inclusive = false })
    local rows = db:scan_by_rowids("users", rowids):to_table()
    assert.equals(2, #rows)
    assert.equals("Grace", rows[1].name)
    assert.equals("Lin", rows[2].name)

    assert.equals("idx_users_name", db:list_indexes("users")[1].name)
    expect_error("IndexAlreadyExists", function()
      db:create_index(backend.index_def({ name = "idx_users_name", table = "users", columns = { "id" } }))
    end)
    db:drop_index("idx_users_name")
    assert.equals(0, #db:list_indexes())
    db:drop_index("idx_users_name", { if_exists = true })
    expect_error("IndexNotFound", function() db:scan_index("idx_users_name") end)
  end)

  it("enforces unique indexes on insert and update", function()
    local db = new_users_backend()
    db:create_index(backend.index_def({ name = "idx_email", table = "users", columns = { "email" }, unique = true }))
    db:insert("users", { id = 1, name = "Ada", email = "ada@example.test" })
    db:insert("users", { id = 2, name = "Grace", email = "grace@example.test" })

    expect_error("ConstraintViolation", function()
      db:insert("users", { id = 3, name = "Other Ada", email = "ada@example.test" })
    end)
    local cursor = db:open_cursor("users")
    cursor:next()
    cursor:next()
    expect_error("ConstraintViolation", function()
      db:update("users", cursor, { email = "ada@example.test" })
    end)
  end)
end)

describe("transactions and savepoints", function()
  it("rolls back and commits snapshots", function()
    local db = new_users_backend()
    local tx = db:begin_transaction()
    db:insert("users", { id = 1, name = "Ada" })
    db:rollback(tx)
    assert.equals(0, #db:scan("users"):to_table())

    tx = db:begin_transaction()
    db:insert("users", { id = 2, name = "Grace" })
    db:commit(tx)
    assert.equals(1, #db:scan("users"):to_table())
  end)

  it("rolls back to a savepoint while keeping it alive", function()
    local db = new_users_backend()
    local tx = db:begin_transaction()
    db:insert("users", { id = 1, name = "Ada" })
    db:create_savepoint("after_ada")
    db:insert("users", { id = 2, name = "Grace" })
    db:rollback_to_savepoint("after_ada")
    assert.equals(1, #db:scan("users"):to_table())
    db:release_savepoint("after_ada")
    db:commit(tx)
  end)
end)

describe("triggers and versions", function()
  it("stores trigger definitions and version fields", function()
    local db = new_users_backend()
    local initial_schema_version = db.schema_version
    local trigger = backend.trigger_def({
      name = "users_ai",
      table = "users",
      timing = "after",
      event = "insert",
      body = "SELECT 1",
    })
    db:create_trigger(trigger)

    assert.is_true(db.schema_version > initial_schema_version)
    assert.equals("users_ai", db:list_triggers("users")[1].name)
    expect_error("TriggerAlreadyExists", function() db:create_trigger(trigger) end)

    db.user_version = 7
    assert.equals(7, db.user_version)
    db:drop_trigger("users_ai")
    assert.equals(0, #db:list_triggers("users"))
    db:drop_trigger("users_ai", { if_exists = true })
    expect_error("TriggerNotFound", function() db:drop_trigger("users_ai") end)
  end)
end)
