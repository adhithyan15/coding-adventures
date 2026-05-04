# mini-sqlite (TypeScript)

TypeScript Level 0 port of the Python `mini-sqlite` facade.

This package is intentionally small: it provides an in-memory database facade,
qmark parameter binding, cursor fetch helpers, and DDL/DML storage. SELECT
queries are delegated to `@coding-adventures/sql-execution-engine`.

## Usage

```typescript
import { connect } from "@coding-adventures/mini-sqlite";

const conn = connect(":memory:");
conn.execute("CREATE TABLE users (id INTEGER, name TEXT)");
conn.executemany("INSERT INTO users VALUES (?, ?)", [
  [1, "Alice"],
  [2, "Bob"],
]);

const rows = conn
  .execute("SELECT name FROM users WHERE id = ?", [1])
  .fetchall();

console.log(rows); // [["Alice"]]
```

## Supported in Level 0

- `connect(":memory:")`
- qmark placeholders (`?`)
- `CREATE TABLE [IF NOT EXISTS]`
- `DROP TABLE [IF EXISTS]`
- `INSERT INTO ... VALUES`
- `UPDATE ... SET ... [WHERE ...]`
- `DELETE FROM ... [WHERE ...]`
- `SELECT ...` through the TypeScript SQL execution engine
- `commit()` and `rollback()` using in-memory snapshots

File-backed connections are reserved for a later port of the SQLite storage
backend and currently raise `NotSupportedError`.
