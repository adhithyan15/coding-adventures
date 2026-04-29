# frozen_string_literal: true

require_relative "test_helper"

MS = CodingAdventures::MiniSqlite

class TestMiniSqlite < Minitest::Test
  def setup
    @conn = MS.connect(":memory:")
  end

  def test_module_constants
    assert_equal "2.0", MS::APILEVEL
    assert_equal 1, MS::THREADSAFETY
    assert_equal "qmark", MS::PARAMSTYLE
  end

  def test_create_insert_select
    @conn.execute("CREATE TABLE users (id INTEGER, name TEXT, active BOOLEAN)")
    @conn.executemany("INSERT INTO users VALUES (?, ?, ?)", [
      [1, "Alice", true],
      [2, "Bob", false],
      [3, "Carol", true]
    ])

    cur = @conn.execute("SELECT name FROM users WHERE active = ? ORDER BY id ASC", [true])

    assert_equal "name", cur.description[0][0]
    assert_equal [["Alice"], ["Carol"]], cur.fetchall
  end

  def test_fetchone_fetchmany_fetchall
    @conn.execute("CREATE TABLE nums (n INTEGER)")
    @conn.executemany("INSERT INTO nums VALUES (?)", [[1], [2], [3]])
    cur = @conn.execute("SELECT n FROM nums ORDER BY n ASC")

    assert_equal [1], cur.fetchone
    assert_equal [[2]], cur.fetchmany(1)
    assert_equal [[3]], cur.fetchall
    assert_nil cur.fetchone
  end

  def test_update_rows
    @conn.execute("CREATE TABLE users (id INTEGER, name TEXT)")
    @conn.executemany("INSERT INTO users VALUES (?, ?)", [[1, "Alice"], [2, "Bob"]])

    cur = @conn.execute("UPDATE users SET name = ? WHERE id = ?", ["Bobby", 2])

    assert_equal 1, cur.rowcount
    assert_equal [["Alice"], ["Bobby"]], @conn.execute("SELECT name FROM users ORDER BY id").fetchall
  end

  def test_delete_rows
    @conn.execute("CREATE TABLE users (id INTEGER, name TEXT)")
    @conn.executemany("INSERT INTO users VALUES (?, ?)", [
      [1, "Alice"],
      [2, "Bob"],
      [3, "Carol"]
    ])

    cur = @conn.execute("DELETE FROM users WHERE id IN (?, ?)", [1, 3])

    assert_equal 2, cur.rowcount
    assert_equal [[2, "Bob"]], @conn.execute("SELECT id, name FROM users").fetchall
  end

  def test_rollback_restores_snapshot
    @conn.execute("CREATE TABLE users (id INTEGER, name TEXT)")
    @conn.commit
    @conn.execute("INSERT INTO users VALUES (?, ?)", [1, "Alice"])
    @conn.rollback

    assert_empty @conn.execute("SELECT * FROM users").fetchall
  end

  def test_commit_keeps_changes
    @conn.execute("CREATE TABLE users (id INTEGER, name TEXT)")
    @conn.execute("INSERT INTO users VALUES (?, ?)", [1, "Alice"])
    @conn.commit
    @conn.rollback

    assert_equal [["Alice"]], @conn.execute("SELECT name FROM users").fetchall
  end

  def test_drop_table
    @conn.execute("CREATE TABLE users (id INTEGER)")
    @conn.execute("DROP TABLE users")

    assert_raises(MS::OperationalError) { @conn.execute("SELECT * FROM users") }
  end

  def test_wrong_parameter_counts
    assert_raises(MS::ProgrammingError) { @conn.execute("SELECT ? FROM t") }
    assert_raises(MS::ProgrammingError) { @conn.execute("SELECT 1 FROM t", [1]) }
  end

  def test_unknown_table
    assert_raises(MS::OperationalError) { @conn.execute("SELECT * FROM missing") }
  end

  def test_unsupported_file_connection
    assert_raises(MS::NotSupportedError) { MS.connect("app.db") }
  end
end
