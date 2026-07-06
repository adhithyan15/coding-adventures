# mini-sqlite (TypeScript)

TypeScript Level 1 in-memory SQL database facade.

All SQL — DDL, DML, and SELECT — is routed through the full Level 1 pipeline:

```
sql-parser → sql-planner → sql-optimizer → sql-codegen → sql-vm
```

This package provides the `Connection` / `Cursor` interface familiar from
Python's DB-API 2.0, qmark parameter binding, and snapshot-based transactions.

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

## Supported in Level 1

- `connect(":memory:")`
- qmark placeholders (`?`)
- `CREATE TABLE [IF NOT EXISTS]`
- `DROP TABLE [IF EXISTS]`
- `INSERT INTO ... VALUES` (multi-row)
- `UPDATE ... SET ... [WHERE ...]`
- `DELETE FROM ... [WHERE ...]`
- `SELECT` with `WHERE`, `GROUP BY`, `HAVING`, `ORDER BY`, `LIMIT`/`OFFSET`,
  `DISTINCT`, aggregate functions (COUNT, SUM, MIN, MAX, AVG), scalar
  functions (UPPER, LOWER, LENGTH, ABS, ROUND, SUBSTR, TRIM, etc.)
- `commit()` and `rollback()` using in-memory snapshots
- `cursor.rowcount` reflects affected rows for DML; `-1` for SELECT/DDL

File-backed connections are reserved for a later port of the SQLite storage
backend and currently raise `NotSupportedError`.
