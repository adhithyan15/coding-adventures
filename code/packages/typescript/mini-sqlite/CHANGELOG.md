# Changelog - @coding-adventures/mini-sqlite

## [0.1.0] - 2026-04-29

### Added
- Initial TypeScript Level 0 mini-sqlite facade.
- In-memory `connect(":memory:")` connections.
- qmark parameter binding.
- `Connection` and `Cursor` APIs with `execute`, `executemany`, and fetch helpers.
- Basic `CREATE TABLE`, `DROP TABLE`, `INSERT`, `UPDATE`, `DELETE`, and `SELECT`.
- Snapshot-backed `commit` and `rollback` behavior.
