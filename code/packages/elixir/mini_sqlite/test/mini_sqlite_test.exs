defmodule CodingAdventures.MiniSqliteTest do
  use ExUnit.Case, async: false

  alias CodingAdventures.MiniSqlite
  alias CodingAdventures.MiniSqlite.Cursor
  alias CodingAdventures.MiniSqlite.Errors.{NotSupportedError, OperationalError, ProgrammingError}

  defp connect! do
    {:ok, conn} = MiniSqlite.connect(":memory:")
    conn
  end

  defp execute!(conn, sql, params \\ []) do
    case MiniSqlite.execute(conn, sql, params) do
      {:ok, cursor} -> cursor
      {:error, error} -> flunk("execute failed: #{Exception.message(error)}")
    end
  end

  test "exposes DB-API style constants" do
    assert MiniSqlite.apilevel() == "2.0"
    assert MiniSqlite.threadsafety() == 1
    assert MiniSqlite.paramstyle() == "qmark"
  end

  test "creates, inserts, and selects rows" do
    conn = connect!()
    execute!(conn, "CREATE TABLE users (id INTEGER, name TEXT, active BOOLEAN)")

    {:ok, _cursor} =
      MiniSqlite.executemany(conn, "INSERT INTO users VALUES (?, ?, ?)", [
        [1, "Alice", true],
        [2, "Bob", false],
        [3, "Carol", true]
      ])

    cursor = execute!(conn, "SELECT name FROM users WHERE active = ? ORDER BY id ASC", [true])
    assert cursor.description == [%{name: "name"}]
    {rows, _cursor} = Cursor.fetchall(cursor)
    assert rows == [["Alice"], ["Carol"]]
  end

  test "fetchone, fetchmany, and fetchall advance immutable cursors" do
    conn = connect!()
    execute!(conn, "CREATE TABLE nums (n INTEGER)")
    {:ok, _cursor} = MiniSqlite.executemany(conn, "INSERT INTO nums VALUES (?)", [[1], [2], [3]])

    cursor = execute!(conn, "SELECT n FROM nums ORDER BY n ASC")
    {row, cursor} = Cursor.fetchone(cursor)
    assert row == [1]
    {rows, cursor} = Cursor.fetchmany(cursor, 1)
    assert rows == [[2]]
    {rows, cursor} = Cursor.fetchall(cursor)
    assert rows == [[3]]
    {row, _cursor} = Cursor.fetchone(cursor)
    assert row == nil
  end

  test "updates rows using the delegated WHERE engine" do
    conn = connect!()
    execute!(conn, "CREATE TABLE users (id INTEGER, name TEXT)")

    {:ok, _cursor} =
      MiniSqlite.executemany(conn, "INSERT INTO users VALUES (?, ?)", [[1, "Alice"], [2, "Bob"]])

    cursor = execute!(conn, "UPDATE users SET name = ? WHERE id = ?", ["Bobby", 2])
    assert cursor.rowcount == 1

    cursor = execute!(conn, "SELECT name FROM users ORDER BY id ASC")
    {rows, _cursor} = Cursor.fetchall(cursor)
    assert rows == [["Alice"], ["Bobby"]]
  end

  test "deletes rows using the delegated WHERE engine" do
    conn = connect!()
    execute!(conn, "CREATE TABLE users (id INTEGER, name TEXT)")

    {:ok, _cursor} =
      MiniSqlite.executemany(conn, "INSERT INTO users VALUES (?, ?)", [
        [1, "Alice"],
        [2, "Bob"],
        [3, "Carol"]
      ])

    cursor = execute!(conn, "DELETE FROM users WHERE id IN (?, ?)", [1, 3])
    assert cursor.rowcount == 2

    cursor = execute!(conn, "SELECT id, name FROM users")
    {rows, _cursor} = Cursor.fetchall(cursor)
    assert rows == [[2, "Bob"]]
  end

  test "rollback restores the open transaction snapshot" do
    conn = connect!()
    execute!(conn, "CREATE TABLE users (id INTEGER, name TEXT)")
    assert :ok = MiniSqlite.commit(conn)
    execute!(conn, "INSERT INTO users VALUES (?, ?)", [1, "Alice"])
    assert :ok = MiniSqlite.rollback(conn)

    cursor = execute!(conn, "SELECT * FROM users")
    {rows, _cursor} = Cursor.fetchall(cursor)
    assert rows == []
  end

  test "commit keeps changes" do
    conn = connect!()
    execute!(conn, "CREATE TABLE users (id INTEGER, name TEXT)")
    execute!(conn, "INSERT INTO users VALUES (?, ?)", [1, "Alice"])
    assert :ok = MiniSqlite.commit(conn)
    assert :ok = MiniSqlite.rollback(conn)

    cursor = execute!(conn, "SELECT name FROM users")
    {rows, _cursor} = Cursor.fetchall(cursor)
    assert rows == [["Alice"]]
  end

  test "autocommit disables rollback snapshots" do
    {:ok, conn} = MiniSqlite.connect(":memory:", autocommit: true)
    execute!(conn, "CREATE TABLE users (id INTEGER)")
    execute!(conn, "INSERT INTO users VALUES (?)", [1])
    assert :ok = MiniSqlite.rollback(conn)

    cursor = execute!(conn, "SELECT id FROM users")
    {rows, _cursor} = Cursor.fetchall(cursor)
    assert rows == [[1]]
  end

  test "drops tables" do
    conn = connect!()
    execute!(conn, "CREATE TABLE users (id INTEGER)")
    execute!(conn, "DROP TABLE users")

    assert {:error, %OperationalError{}} = MiniSqlite.execute(conn, "SELECT * FROM users")
  end

  test "wrong parameter counts are programming errors" do
    conn = connect!()
    assert {:error, %ProgrammingError{}} = MiniSqlite.execute(conn, "SELECT ? FROM t")
    assert {:error, %ProgrammingError{}} = MiniSqlite.execute(conn, "SELECT 1 FROM t", [1])
  end

  test "rejects file-backed connections in Level 0" do
    assert {:error, %NotSupportedError{}} = MiniSqlite.connect("app.db")
  end
end
