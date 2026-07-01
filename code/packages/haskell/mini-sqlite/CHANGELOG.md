# Changelog

All notable changes to this package will be documented in this file.

## [0.2.1] - 2026-07-01

### Security

- **`roundHalfAway` overflow fix (CRITICAL)**: replaced `Int` intermediate
  arithmetic with `Integer` (arbitrary-precision) to prevent silent integer
  overflow corrupting ROUND results.  Added negative-digits support (SQLite
  ROUND semantics) and clamped `digits` to [-15, 15].
- **`likeMatch` ReDoS fix (HIGH)**: consecutive `%` wildcards in a LIKE pattern
  are collapsed to a single `%` before recursion, eliminating exponential
  backtracking on crafted inputs such as `col LIKE '%a%a%a%a%b'`.
- **`formatParam` null-byte rejection (MEDIUM)**: `SqlText` parameters
  containing `\NUL` now produce an explicit error rather than emitting a
  truncated string literal that could confuse downstream parsers.

### Added

- `fetchone_test`, `fetchmany_test`, `fetchall_test`, `fetchall_empty_test`
  operations in `ConformanceSpec`, enabling fixture 15 to pass.

## [0.2.0] - 2026-07-01

### Changed
- **Graduate to Level 1**: `execute` now routes every SQL statement through the
  full pipeline: hand-rolled tokeniser → `SqlPlanner.plan` →
  `SqlOptimizer.optimize` → `SqlCodegen.compile` → `SqlVm.executeWithRef`.
- `Connection` now wraps `IORef InMemoryBackend` for the live state plus a
  snapshot `IORef` for snapshot-based manual-commit rollback semantics.
- `cabal.project` now lists all transitive local dependencies:
  `sql-backend`, `sql-planner`, `sql-optimizer`, `sql-codegen`, `sql-vm`.
- Version bumped to `0.2.0`.

### Added
- `ConformanceSpec`: 24 conformance fixtures from
  `code/specs/mini-sqlite-conformance/fixtures/` now run as part of the test
  suite.
- `evalScalarSelect`/`evalScalarExpr`/`evalScalarFunc` for SELECT without FROM
  (literal expressions, `LENGTH`, `UPPER`, `LOWER`, `SUBSTR`, `TRIM`, `LTRIM`,
  `RTRIM`, `REPLACE`, `CONCAT` via `||`, `COALESCE`, `IFNULL`).
- `backendSchemaProvider` builds a `SqlPlanner.SchemaProvider` directly from
  `SqlBackend.columns`.
- `SqlVm.executeWithRef` added to `sql-vm` (exposes mutated backend to callers).
- `CallBuiltin String Int` instruction added to `sql-codegen` and `sql-vm` so
  that scalar functions (`LOWER`, `UPPER`, `LENGTH`, `SUBSTR`, `TRIM`, etc.)
  work in FROM-clause queries, not just SELECT-without-FROM.

### Fixed
- `SELECT without FROM` (e.g. `SELECT 1+1`, `SELECT LENGTH('x')`) is detected
  before planning and evaluated via `evalScalarSelect`.

## [0.1.0] - 2026-05-02

- Added a Level 0 in-memory mini-sqlite facade for Haskell.
