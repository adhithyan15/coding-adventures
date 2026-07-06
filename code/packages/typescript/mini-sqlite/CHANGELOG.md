# Changelog - @coding-adventures/mini-sqlite

## [0.2.0] - 2026-07-06

### Changed
- **Level 1 upgrade**: replaced the Level 0 hybrid engine (regex-parsed DML +
  `sql-execution-engine` for SELECT) with the full Level 1 pipeline:
  `sql-parser → sql-planner → sql-optimizer → sql-codegen → sql-vm`.
- `rowsAffected` now correctly reflects the number of rows changed by each DML
  statement (previously returned 0 for all DML; now uses `result.rowsAffected`
  from the VM).
- Error translation updated: `VmError` and `CodegenError` from the new pipeline
  are mapped to `OperationalError` or `ProgrammingError` as appropriate.
- Removed `isDmlPlan` helper (superseded by `result.rowsAffected` sentinel).

### Added
- `rowsAffected` in `StatementResult` reflects INSERT/UPDATE/DELETE row counts.
- Missing-table queries now correctly raise `OperationalError` instead of
  returning empty results silently.

## [0.1.1] - 2026-06-30

### Fixed
- Removed `\s*;?\s*$` trailer from five DDL/DML parsing regexes; trailing
  semicolon + whitespace are now stripped imperatively before matching.
  The old form caused polynomial backtracking on adversarial whitespace-only
  input (codeql `js/polynomial-redos`).

## [0.1.0] - 2026-04-29

### Added
- Initial TypeScript Level 0 mini-sqlite facade.
- In-memory `connect(":memory:")` connections.
- qmark parameter binding.
- `Connection` and `Cursor` APIs with `execute`, `executemany`, and fetch helpers.
- Basic `CREATE TABLE`, `DROP TABLE`, `INSERT`, `UPDATE`, `DELETE`, and `SELECT`.
- Snapshot-backed `commit` and `rollback` behavior.
