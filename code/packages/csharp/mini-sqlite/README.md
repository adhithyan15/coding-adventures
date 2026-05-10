# CodingAdventures.MiniSqlite

`CodingAdventures.MiniSqlite` is the C# Level 0 port of the mini-sqlite facade. It provides an in-memory connection and cursor API with qmark binding and transaction snapshots.

## Scope

- `MiniSqlite.Connect(":memory:")`
- `CREATE TABLE`, `DROP TABLE`, `INSERT`, `UPDATE`, `DELETE`
- simple `SELECT` queries with column projection, `WHERE`, and `ORDER BY`
- qmark parameter binding
- `Commit`, `Rollback`, and close-time rollback snapshots when autocommit is disabled

File-backed SQLite pages are out of scope for Level 0. Opening anything other than `:memory:` raises a `NotSupportedError`.

## Example

```csharp
using CodingAdventures.MiniSqlite;

using var conn = MiniSqlite.Connect(":memory:");
conn.Execute("CREATE TABLE users (id INTEGER, name TEXT)");
conn.Execute("INSERT INTO users VALUES (?, ?)", 1, "Alice");

var cursor = conn.Execute("SELECT name FROM users");
var rows = cursor.FetchAll();
```
