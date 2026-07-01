# Changelog — sql-vm (Kotlin)

All notable changes to this package are documented here.

## [0.1.0] — 2026-06-30

### Added

- Initial implementation of `SqlVm.execute(program, backend)` — the dispatch-loop
  stack-machine VM for the mini-sqlite Level 1 pipeline.
- `QueryResult` data class with `columns`, `rows`, and `rowsAffected` fields.
- Complete `Instruction` dispatch covering:
  - Stack ops: `LoadConst`, `LoadColumn`, `LoadParam`, `LoadGroupKey`, `Pop`
  - Arithmetic binary ops: `ADD`, `SUB`, `MUL`, `DIV`, `MOD`, `CONCAT`
  - Comparison binary ops: `EQ`, `NEQ`, `LT`, `LTE`, `GT`, `GTE`
  - Logic binary ops: `AND`, `OR` (three-valued / SQL semantics)
  - Unary ops: `NEG`, `NOT`
  - Predicate tests: `IsNull`, `IsNotNull`, `Between`, `Like`, `InList`
  - Scan lifecycle: `OpenScan`, `AdvanceCursor`, `JumpIfExhausted`, `CloseScan`
  - Row emission: `BeginRow`, `EmitColumn`, `EmitRow`
  - Aggregation: `InitAgg`, `UpdateAgg`, `FinalizeAgg`, `SaveGroupKey`,
    `LoadGroupKey`, `AdvanceGroup`
  - Control flow: `Label` (no-op), `Jump`, `JumpIfTrue`, `JumpIfFalse`, `Halt`
  - DDL: `CreateTableInstr`, `DropTableInstr`
  - DML: `InsertRow`, `UpdateRows`, `DeleteRows`
  - Transactions: `BeginTransaction`, `CommitTransaction`, `RollbackTransaction`
  - Post-processing: `SortResult`, `DistinctResult`, `LimitResult`
- SQL three-valued logic (NULL propagation) for all binary and unary operators.
- SQL LIKE matching via regex conversion (`%` → `.*`, `_` → `.`), case-insensitive.
- GROUP BY aggregation with per-group accumulator tables.
- Stable sort for `ORDER BY` (preserves insertion order for equal keys).
- NULLS FIRST / NULLS LAST placement in `SortResult`.
- 45+ JUnit 5 tests covering all instruction variants, NULL semantics, aggregates,
  ORDER BY, DISTINCT, LIMIT/OFFSET, DML, DDL, and transaction behaviour.
- JaCoCo coverage configured at ≥80% line coverage.
- Knuth-style literate comments throughout `SqlVm.kt`.
- `BUILD` / `BUILD_windows` scripts that pre-build all four sibling JARs in
  leaf-to-root order before running tests.
- `required_capabilities.json` documenting the package's capability requirements.
