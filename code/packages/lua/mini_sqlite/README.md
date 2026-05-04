# coding-adventures-mini-sqlite

`coding-adventures-mini-sqlite` is the Lua Level 0 port of the mini-sqlite facade. It provides an in-memory connection and cursor API while delegating `SELECT` execution to `coding_adventures.sql_execution_engine`.

## Scope

- `mini_sqlite.connect(":memory:")`
- `CREATE TABLE`, `DROP TABLE`, `INSERT`, `UPDATE`, `DELETE`
- `SELECT` queries supported by the Lua SQL execution engine
- qmark parameter binding
- `commit`, `rollback`, and close-time rollback snapshots when autocommit is disabled

File-backed SQLite pages are out of scope for Level 0. Opening anything other than `:memory:` returns a `NotSupportedError`.

## Example

```lua
local mini = require("coding_adventures.mini_sqlite")

local conn = assert(mini.connect(":memory:"))
assert(conn:execute("CREATE TABLE users (id INTEGER, name TEXT)"))
assert(conn:execute("INSERT INTO users VALUES (?, ?)", {1, "Alice"}))

local cursor = assert(conn:execute("SELECT name FROM users"))
local rows = cursor:fetchall()
assert(rows[1][1] == "Alice")
```
