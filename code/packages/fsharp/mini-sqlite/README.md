# CodingAdventures.MiniSqlite.FSharp

Level 0 F# port of the mini-sqlite facade. It exposes DB-API-inspired
connection and cursor APIs backed by an in-memory table store.

Supported in this first slice:

- `MiniSqlite.ApiLevel`, `ThreadSafety`, and `ParamStyle`
- `MiniSqlite.Connect(":memory:")`
- qmark parameter binding
- `CREATE TABLE`, `DROP TABLE`, `INSERT`, `UPDATE`, `DELETE`
- `SELECT` with projection, `WHERE`, and `ORDER BY`
- cursor `FetchOne`, `FetchMany`, and `FetchAll`
- transaction snapshots with `Commit` and `Rollback`

File-backed databases, joins, indexes, and full SQLite semantics are left for
later levels.
