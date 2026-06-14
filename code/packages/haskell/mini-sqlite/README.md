# mini-sqlite

Level 0 Haskell port of the mini-sqlite facade. It exposes an `Either`/`IO`
API backed by in-memory tables.

Supported in this first slice:

- `apiLevel`, `threadSafety`, and `paramStyle`
- `connect ":memory:"`
- qmark parameter binding
- `CREATE TABLE`, `DROP TABLE`, `INSERT`, `UPDATE`, `DELETE`
- `SELECT` with projection, `WHERE`, and `ORDER BY`
- cursor `fetchOne`, `fetchMany`, and `fetchAll`
- transaction snapshots with `commit` and `rollback`

File-backed databases, joins, indexes, and full SQLite semantics are left for
later levels.
