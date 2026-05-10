# MiniSqlite

Level 0 Swift port of the mini-sqlite facade. It exposes a lightweight
DB-API-inspired connection and cursor API backed by in-memory tables.

Supported in this first slice:

- `MiniSqlite.apiLevel`, `threadSafety`, and `paramStyle`
- `MiniSqlite.connect(":memory:")`
- qmark parameter binding
- `CREATE TABLE`, `DROP TABLE`, `INSERT`, `UPDATE`, `DELETE`
- `SELECT` with projection, `WHERE`, and `ORDER BY`
- cursor `fetchOne`, `fetchMany`, and `fetchAll`
- transaction snapshots with `commit` and `rollback`

File-backed databases, joins, indexes, and full SQLite semantics are left for
later levels.
