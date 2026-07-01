# Changelog — CodingAdventures.SqlVm.FSharp

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [0.1.0] — 2026-06-30

### Added

- `QueryResult` record: `Columns`, `Rows`, `RowsAffected`.
- `SqlVm.execute : Program -> Backend -> QueryResult` — the single public entry point.
- Full stack-machine dispatch loop over all `Instruction` cases produced by `sql-codegen`:
  - Constants / columns: `LoadConst`, `LoadColumn`, `LoadParam`, `LoadGroupKey`, `LoadOuterColumn`, `Pop`.
  - Arithmetic / comparison: `BinaryOpInstr` (Add, Sub, Mul, Div, Mod, Concat, Eq, Neq, Lt, Lte, Gt, Gte).
  - Three-valued logic: `BinaryOpInstr And/Or` with short-circuit semantics per SQL standard.
  - Unary: `UnaryOpInstr` (Neg, Not).
  - Predicates: `IsNull`, `IsNotNull`, `Between`, `Like`, `InList`.
  - Scan / cursor: `OpenScan`, `AdvanceCursor`, `JumpIfExhausted`, `CloseScan`.
  - Row construction: `BeginRow`, `EmitColumn`, `EmitRow`.
  - Aggregation: `InitAgg`, `UpdateAgg`, `FinalizeAgg`, `SaveGroupKey`, `AdvanceGroup`.
  - Control flow: `Label`, `Jump`, `JumpIfTrue`, `JumpIfFalse`, `Halt`.
  - DDL: `CreateTable`, `DropTable`.
  - DML: `InsertRow`, `UpdateRows`, `DeleteRows`.
  - Transactions: `BeginTransaction`, `CommitTransaction`, `RollbackTransaction`.
  - Post-processing: `SortResult`, `DistinctResult`, `LimitResult`.
- O(1) label resolution via pre-scanned `Map<string,int>` label index.
- SQL three-valued logic helpers (`isTruthy`, NULL propagation in arithmetic/comparison).
- SQL LIKE matching via .NET `Regex` conversion (`%` → `.*`, `_` → `.`).
- Aggregate accumulators for COUNT(*), COUNT(col), SUM, AVG, MIN, MAX with correct NULL handling.
- 56 xUnit tests covering all instruction categories.
- 80%+ line coverage enforced via `coverlet.msbuild`.
