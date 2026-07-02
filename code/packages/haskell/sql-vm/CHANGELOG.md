# Changelog — sql-vm (Haskell)

All notable changes to this package are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

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
