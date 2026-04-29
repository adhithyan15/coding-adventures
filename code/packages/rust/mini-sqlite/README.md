# coding-adventures-mini-sqlite

`coding-adventures-mini-sqlite` is the Rust Level 0 port of the mini-sqlite facade. It exposes a small DB-API-inspired surface over an in-memory table store and delegates `SELECT` execution to `coding-adventures-sql-execution-engine`.

## Scope

- `connect(":memory:")`
- `CREATE TABLE`, `DROP TABLE`, `INSERT`, `UPDATE`, `DELETE`
- `SELECT` queries supported by the Rust SQL execution engine
- qmark parameter binding with `SqlValue` parameters
- `commit`, `rollback`, and close-time rollback snapshots when autocommit is disabled

File-backed SQLite pages are intentionally out of scope for Level 0. Opening any database name other than `:memory:` returns `MiniSqliteError::NotSupportedError`.

## Example

```rust
use coding_adventures_mini_sqlite::{connect, int, text, boolean};

let conn = connect(":memory:").unwrap();
conn.execute("CREATE TABLE users (id INTEGER, name TEXT, active BOOLEAN)", &[]).unwrap();
conn.execute(
    "INSERT INTO users VALUES (?, ?, ?)",
    &[int(1), text("Alice"), boolean(true)],
).unwrap();

let mut cursor = conn
    .execute("SELECT name FROM users WHERE active = ?", &[boolean(true)])
    .unwrap();
assert_eq!(cursor.fetchall(), vec![vec![text("Alice")]]);
```
