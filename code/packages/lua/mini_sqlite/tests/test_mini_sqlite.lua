local mini = require("coding_adventures.mini_sqlite")

local function connect()
  local conn, err = mini.connect(":memory:")
  assert.is_nil(err)
  return conn
end

describe("mini_sqlite", function()
  it("exposes DB-API style constants", function()
    assert.equals("2.0", mini.apilevel)
    assert.equals(1, mini.threadsafety)
    assert.equals("qmark", mini.paramstyle)
  end)

  it("creates, inserts, and selects rows", function()
    local conn = connect()
    assert(conn:execute("CREATE TABLE users (id INTEGER, name TEXT, active BOOLEAN)"))
    assert(conn:executemany("INSERT INTO users VALUES (?, ?, ?)", {
      {1, "Alice", true},
      {2, "Bob", false},
      {3, "Carol", true},
    }))

    local cursor = assert(conn:execute("SELECT name FROM users WHERE active = ? ORDER BY id ASC", {true}))
    assert.equals("name", cursor.description[1].name)
    local rows = cursor:fetchall()
    assert.equals("Alice", rows[1][1])
    assert.equals("Carol", rows[2][1])
  end)

  it("fetches incrementally", function()
    local conn = connect()
    assert(conn:execute("CREATE TABLE nums (n INTEGER)"))
    assert(conn:executemany("INSERT INTO nums VALUES (?)", {{1}, {2}, {3}}))
    local cursor = assert(conn:execute("SELECT n FROM nums ORDER BY n ASC"))

    assert.equals(1, cursor:fetchone()[1])
    assert.equals(2, cursor:fetchmany(1)[1][1])
    assert.equals(3, cursor:fetchall()[1][1])
    assert.is_nil(cursor:fetchone())
  end)

  it("updates and deletes rows", function()
    local conn = connect()
    assert(conn:execute("CREATE TABLE users (id INTEGER, name TEXT)"))
    assert(conn:executemany("INSERT INTO users VALUES (?, ?)", {{1, "Alice"}, {2, "Bob"}, {3, "Carol"}}))

    local updated = assert(conn:execute("UPDATE users SET name = ? WHERE id = ?", {"Bobby", 2}))
    assert.equals(1, updated.rowcount)

    local deleted = assert(conn:execute("DELETE FROM users WHERE id IN (?, ?)", {1, 3}))
    assert.equals(2, deleted.rowcount)

    local cursor = assert(conn:execute("SELECT id, name FROM users"))
    local rows = cursor:fetchall()
    assert.equals(2, rows[1][1])
    assert.equals("Bobby", rows[1][2])
  end)

  it("rolls back and commits snapshots", function()
    local conn = connect()
    assert(conn:execute("CREATE TABLE users (id INTEGER, name TEXT)"))
    assert(conn:commit())
    assert(conn:execute("INSERT INTO users VALUES (?, ?)", {1, "Alice"}))
    assert(conn:rollback())
    assert.equals(0, #assert(conn:execute("SELECT * FROM users")):fetchall())

    assert(conn:execute("INSERT INTO users VALUES (?, ?)", {1, "Alice"}))
    assert(conn:commit())
    assert(conn:rollback())
    assert.equals(1, #assert(conn:execute("SELECT * FROM users")):fetchall())
  end)

  it("rejects file-backed connections", function()
    local conn, err = mini.connect("app.db")
    assert.is_nil(conn)
    assert.equals("NotSupportedError", err.kind)
  end)
end)
