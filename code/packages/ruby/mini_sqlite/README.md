# mini-sqlite (Ruby)

Ruby Level 0 port of the Python `mini-sqlite` facade.

The package provides an in-memory database facade with qmark binding, basic
DDL/DML, cursors, and transaction snapshots. SELECT queries are delegated to
the existing Ruby `sql_execution_engine` package.

```ruby
require "coding_adventures_mini_sqlite"

conn = CodingAdventures::MiniSqlite.connect(":memory:")
conn.execute("CREATE TABLE users (id INTEGER, name TEXT)")
conn.executemany("INSERT INTO users VALUES (?, ?)", [[1, "Alice"], [2, "Bob"]])

rows = conn.execute("SELECT name FROM users WHERE id = ?", [1]).fetchall
puts rows.inspect # [["Alice"]]
```

File-backed connections are reserved for a later storage backend port and
currently raise `NotSupportedError`.
