# mini-sqlite

`mini-sqlite` is the Kotlin Level 0 port of the mini-sqlite facade. It provides an in-memory connection and cursor API with qmark binding and transaction snapshots.

## Scope

- `MiniSqlite.connect(":memory:")`
- `CREATE TABLE`, `DROP TABLE`, `INSERT`, `UPDATE`, `DELETE`
- simple `SELECT` queries with column projection, `WHERE`, and `ORDER BY`
- qmark parameter binding
- `commit`, `rollback`, and close-time rollback snapshots when autocommit is disabled

File-backed SQLite pages are out of scope for Level 0. Opening anything other than `:memory:` raises a `NotSupportedError`.

## Example

```kotlin
val conn = MiniSqlite.connect(":memory:")
conn.execute("CREATE TABLE users (id INTEGER, name TEXT)")
conn.execute("INSERT INTO users VALUES (?, ?)", listOf(1, "Alice"))

val cursor = conn.execute("SELECT name FROM users")
val rows = cursor.fetchall()
```
