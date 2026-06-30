# Changelog - coding_adventures_mini_sqlite

## [0.1.1] - 2026-06-30

### Fixed
- Removed `\s*;?\s*\z` trailer from four DDL/DML parsing regexes; trailing
  semicolon + whitespace are now stripped imperatively before matching.
  The old form caused polynomial backtracking on adversarial whitespace-only
  input (codeql `rb/polynomial-redos`).

## [0.1.0] - 2026-04-29

### Added
- Initial Ruby Level 0 mini-sqlite facade.
- In-memory `connect(":memory:")` connections.
- qmark parameter binding.
- Basic `CREATE TABLE`, `DROP TABLE`, `INSERT`, `UPDATE`, `DELETE`, and `SELECT`.
- Cursor fetch helpers and snapshot-backed `commit` / `rollback`.
