# Changelog - mini-sqlite (Go)

## [0.1.0] - 2026-04-29

### Added
- Initial Go Level 0 mini-sqlite facade.
- In-memory `Connect(":memory:")` connections.
- qmark parameter binding.
- Basic `CREATE TABLE`, `DROP TABLE`, `INSERT`, `UPDATE`, `DELETE`, and `SELECT`.
- Cursor fetch helpers and snapshot-backed `Commit` / `Rollback`.
