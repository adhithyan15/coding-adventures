# Changelog — sql-vm (Haskell)

All notable changes to this package are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.2.0] — 2026-07-01

### Security

- **`roundHalfAwayVm` overflow fix (CRITICAL)**: replaced `Int` intermediate
  arithmetic with `Integer` (arbitrary-precision) to prevent silent integer
  overflow corrupting ROUND results for large inputs.  Added negative-digits
  support (SQLite ROUND semantics) and clamped `digits` to [-15, 15] to prevent
  a `"Negative exponent"` runtime exception on user-controlled input.
- **`InsertRow` error handling (HIGH)**: replaced `error` (impure exception) with
  `liftIO (throwIO (userError …))` (proper IO exception).  The existing
  `catch`-based wrapper in `MiniSqlite` already handled IO exceptions; this
  change makes the handling structurally sound rather than relying on an ambient
  catch that could be removed by future refactoring.

## [0.1.1.0] — 2026-07-01

### Added

- **`executeWithRef :: Program -> IORef InMemoryBackend -> IO (QueryResult, InMemoryBackend)`**
  — new export that accepts a caller-supplied `IORef` for the backend and
  returns both the `QueryResult` and the final (possibly mutated) backend.
  Enables callers (e.g. `mini-sqlite`) to capture DML/DDL side-effects.
- **`CallBuiltin` dispatch** in the VM's `dispatch` function:
  pops `arity` arguments, reverses to restore left-to-right order, and calls
  `evalBuiltin`.
- **`evalBuiltin :: String -> [SqlValue] -> SqlValue`** — evaluates built-in
  scalar functions: `length`, `upper`, `lower`, `substr` (1-indexed, 2- or
  3-arg), `trim`, `ltrim`, `rtrim`, `replace`, `abs`, `concat`, `coalesce`,
  `ifnull`.  Unknown names return `SqlNull`.
- **`replaceAllVm`** helper for non-overlapping string replacement (used by
  `evalBuiltin "replace"`).
- Imports extended: `Data.Char.isSpace`, `Data.Char.toUpper`,
  `Data.List.dropWhileEnd`, `Data.List.isPrefixOf`.

### Changed

- `execute` now delegates to `executeWithRef` internally.

## [0.1.0.0] — 2026-07-01

### Added

- **`SqlVm` module** — exports `QueryResult(..)` and `execute`.
- **`execute :: Program -> InMemoryBackend -> IO QueryResult`** — public entry
  point; wraps the backend in an `IORef`, builds the label index, runs the
  dispatch loop, and applies post-processing.
- **Stack machine dispatch** — handles all `Instruction` constructors from
  `sql-codegen`:
  - `LoadConst`, `LoadNull`, `LoadColumn`, `LoadParam` (NULL stub), `Pop`
  - `BinaryOpInstr` — arithmetic (+, -, *, /, %), comparison (=, <>, <, <=,
    >, >=), logical (AND, OR), concatenation (||)
  - `UnaryOpInstr` — negation (Neg), boolean inversion (Not)
  - `IsNullInstr`, `IsNotNullInstr`
  - `BetweenInstr` — inclusive range check with NULL propagation
  - `LikeInstr` — two-pointer LIKE matcher (no Regex; ReDoS-safe)
  - `InList` — SQL IN membership with NULL-in-list semantics
  - `OpenScan`, `AdvanceCursor`, `JumpIfExhausted`, `CloseScan`
  - `BeginRow`, `EmitColumn`, `EmitRow`
  - `InitAgg`, `UpdateAgg`, `FinalizeAgg` — aggregate accumulators for
    COUNT(*), COUNT, SUM, AVG, MIN, MAX
  - `SaveGroupKey`, `AdvanceGroup`, `LoadGroupKey` — Level 1 stubs
  - `Label`, `Jump`, `JumpIfTrue`, `JumpIfFalse`, `Halt`
  - `CreateTableInstr`, `DropTableInstr` — DDL via backend
  - `InsertRow` — DML via backend; increments `rowsAffected`
  - `UpdateRows`, `DeleteRows` — Level 1 stubs
  - `BeginTransaction`, `CommitTransaction`, `RollbackTransaction` — no-ops
  - `SortResult`, `DistinctResult`, `LimitResult` — post-processing directives
- **Three-valued SQL logic** — AND/OR short-circuit on FALSE/TRUE even when
  the other operand is NULL.
- **Cursor peek pattern** — `JumpIfExhausted` peeks the iterator; `AdvanceCursor`
  clears the peek so the loop works correctly without double-advancing.
- **Label pre-indexing** — `buildLabelIndex` builds a `Map String Int` for
  O(1) jump resolution.
- **Post-processing pipeline** — Sort → Distinct → Limit applied in
  `buildResult` after the scan loop completes.
- **`compareSqlValues` ordering** — NULL < BOOL < INTEGER/REAL < TEXT < BLOB.
- **`plannerColToBackendCol`** — converts `SqlPlanner.ColumnDef` to
  `SqlBackend.ColumnDef` for DDL execution.
- **50+ hspec tests** in `test/SqlVmSpec.hs` covering all instruction families,
  NULL semantics, LIKE/BETWEEN/IN, aggregates, sort/limit/distinct, DDL and DML.
- `sql-vm.cabal`, `cabal.project`, `BUILD`, `BUILD_windows`,
  `required_capabilities.json`.
