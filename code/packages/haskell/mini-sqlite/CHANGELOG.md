# Changelog

All notable changes to this package will be documented in this file.

## [0.2.6] - 2026-07-01

### Fixed

- **Cursor alias mismatch — all column reads returned `SqlNull` (runtime
  bug)**: the full-pipeline path (`sql-codegen` → `sql-vm`) was affected by
  a mismatch between how `sql-codegen` opened cursors and how `sql-planner`
  resolved column references.  The planner resolves bare column names as
  `Column (Just tableName) col` (using the table name as scope alias when no
  AS alias is given).  The codegen therefore emits `LoadColumn (Just
  tableName) col`.  However `compileScanLoop` was emitting `OpenScan table
  Nothing`, which stored the row under cursor key `""`.  `LoadColumn (Just
  tableName)` looked up `vmCurrentRow[tableName]` which didn't exist, so
  every column read returned `SqlNull`.  Fixed in `sql-codegen 0.1.2.0` by
  normalising the cursor alias to `Just tableName` when no explicit alias is
  present.  After this fix, INSERT + SELECT round-trips return real values
  instead of `SqlNull`.

- **`SELECT *` returned rows with zero columns (runtime bug)**: `OutputStar`
  compiled to a bare `LoadConst (LitText "*")` marker which was never
  converted to `EmitColumn` calls.  `EmitRow` therefore committed an empty
  row buffer, giving `[[]]` for every `SELECT * FROM t`.  Fixed in `sql-vm
  0.1.4.0`: `EmitRow` now expands the `"*"` marker by copying all columns
  from the current cursor row into the output buffer.

- **Output column order was alphabetical instead of SELECT-list order
  (runtime bug)**: `buildResult` in `sql-vm` derived column names via
  `Map.keys` of the first output row, which is sorted alphabetically.  Queries
  like `SELECT id, name, age FROM t` returned `["age","id","name"]` instead
  of `["id","name","age"]`.  Fixed in `sql-vm 0.1.4.0` by tracking emit order
  per-row and capturing it into `vmOutputColumns`.

- **Aggregate `AS` aliases ignored — always reported as `_agg0` etc.
  (runtime bug)**: `collectAggs` in `sql-planner` assigns synthetic aliases
  `_agg0`, `_agg1` to aggregate items.  The codegen was passing these through
  to `EmitColumn` even when the outer `SELECT` clause had an explicit `AS`
  alias.  Fixed in `sql-codegen 0.1.2.0` by forwarding the outer `Project`
  column list to `compileAggregateQuery` so that user-supplied aliases override
  the synthetic ones.

## [0.2.5] - 2026-07-01

### Fixed

- **`InsertRow` VM bug — INSERT values silently discarded (runtime)**: the
  `sql-vm` package's `InsertRow` handler was reading from `vmRowBuffer` (the
  SELECT-row accumulator) instead of popping the stack values that `compileInsert`
  had pushed.  Every INSERT therefore inserted an empty row, causing all subsequent
  SELECTs to return `SqlNull`/empty results.  Fixed in `sql-vm` 0.1.3.0.

- **`ConformanceSpec` fixture path off by one (runtime)**: `fixtureDir` was set
  to `../../../../specs/...` (four levels up from the package root), which points
  to the repository root where no `specs/` directory exists.  The correct relative
  path from `code/packages/haskell/mini-sqlite` is three levels up to `code/`,
  then into `specs/`.  Fixed to `../../../specs/mini-sqlite-conformance/fixtures`.

## [0.2.4] - 2026-07-01

### Fixed

- **`ConformanceSpec` `String`/`T.Text` mismatch (GHC compile error)**: the
  conformance test file used un-adorned string literals (e.g. `"op"`, `"execute"`,
  `""`) in contexts that required `Data.Text.Text`.  Added
  `{-# LANGUAGE OverloadedStrings #-}` to `ConformanceSpec.hs` so that all
  string literals are polymorphic and unify with `T.Text` at the use sites
  (`getStr`, `Map.lookup`, case-expression patterns, and the empty-string default
  in `getStr`).

## [0.2.3] - 2026-07-01

### Fixed

- **`parseInsert` type error (GHC compile error)**: `parseValueRows` returns
  `Either MiniSqliteError ([[SqlExpr]], [Token])` — a pair of (rows, remaining
  tokens).  The monadic bind in `parseInsert` was discarding the tuple structure,
  binding `rows` to the entire `([[SqlExpr]], [Token])` pair instead of just the
  `[[SqlExpr]]` part.  Fixed by destructuring: `(rows, _) <- parseValueRows ...`
  at both the column-list (`INSERT INTO t (c1,c2) VALUES (...)`) and no-column-
  list (`INSERT INTO t VALUES (...)`) code paths.

## [0.2.2] - 2026-07-01

### Fixed

- **Ambiguous `Column` name (GHC compile error)**: `MiniSqlite` defines its own
  `Column` record type for cursor descriptions.  The unqualified `SqlExpr(..)`
  import also brought in `SqlPlanner.Column` (the `SqlExpr` constructor), making
  every occurrence ambiguous.  Fixed by listing the `SqlExpr` constructors
  explicitly — omitting `Column` — and using the qualified alias `P.Column` at
  the four sites that construct or pattern-match on `SqlPlanner.Column`.

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
