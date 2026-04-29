# coding_adventures_mini_sqlite

`coding_adventures_mini_sqlite` is the Elixir Level 0 port of the mini-sqlite facade. It exposes an in-memory SQL connection and cursor API, while reusing `CodingAdventures.SqlExecutionEngine` for `SELECT`.

## Scope

- `CodingAdventures.MiniSqlite.connect(":memory:")`
- `CREATE TABLE`, `DROP TABLE`, `INSERT`, `UPDATE`, `DELETE`
- `SELECT` queries supported by the Elixir SQL execution engine
- qmark parameter binding
- `commit`, `rollback`, and close-time rollback snapshots when autocommit is disabled

Opening anything other than `:memory:` returns `{:error, %NotSupportedError{}}` in Level 0.

## Example

```elixir
alias CodingAdventures.MiniSqlite
alias CodingAdventures.MiniSqlite.Cursor

{:ok, conn} = MiniSqlite.connect(":memory:")
{:ok, _} = MiniSqlite.execute(conn, "CREATE TABLE users (id INTEGER, name TEXT)", [])
{:ok, _} = MiniSqlite.execute(conn, "INSERT INTO users VALUES (?, ?)", [1, "Alice"])
{:ok, cursor} = MiniSqlite.execute(conn, "SELECT name FROM users", [])
{rows, _cursor} = Cursor.fetchall(cursor)
rows == [["Alice"]]
```
