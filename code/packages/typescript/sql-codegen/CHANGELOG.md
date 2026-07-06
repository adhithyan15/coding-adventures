# Changelog - @coding-adventures/sql-codegen

## [0.1.0] - 2026-07-06

### Added
- Initial TypeScript implementation of the SQL code generator.
- `compile(plan: LogicalPlan): Program` — translates an optimized `LogicalPlan`
  into a flat `Program` (list of IR instructions + label index + result schema).
- Full IR instruction set defined in `ir.ts`:
  - Stack: `LoadConst`, `LoadNull`, `LoadColumn`, `Pop`
  - Arithmetic / logic: `BinaryOp`, `UnaryOp`, `CallFunc`, `IsNullInstr`,
    `IsNotNullInstr`, `BetweenInstr`, `LikeInstr`, `InList`, `Coalesce`
  - Table scan: `OpenScan`, `AdvanceCursor`, `JumpIfExhausted`, `CloseScan`
  - Row output: `BeginRow`, `EmitColumn`, `EmitRow`, `SetResultSchema`
  - Aggregates: `InitAgg`, `UpdateAgg`, `FinalizeAgg`, `SaveGroupKey`,
    `LoadGroupKey`, `AdvanceGroup`, `JumpIfGroupsDone`
  - Post-processing: `SortResult`, `DistinctResult`, `LimitResult`
  - DML: `InsertRow`, `UpdateRows`, `DeleteRows`
  - DDL: `CreateTable`, `DropTable`
  - Transactions: `BeginTransaction`, `CommitTransaction`, `RollbackTransaction`
  - Control flow: `Label`, `Jump`, `JumpIfTrue`, `JumpIfFalse`, `Halt`
- `buildLabelIndex(instructions)` utility to build the label map.
- Support for all `LogicalPlan` node types:
  - `ScanNode`, `FilterNode`, `ProjectNode`, `SortNode`, `LimitNode`,
    `DistinctNode`, `AggregateNode`, `HavingNode`, `JoinNode`
  - DML: `InsertNode`, `UpdateNode`, `DeleteNode`
  - DDL: `CreateTableNode`, `DropTableNode`
  - `TransactionNode`
- Aggregate compilation uses a two-phase approach:
  - Phase 1: scan all rows, accumulate into group slots via `UpdateAgg`
  - Phase 2: emit one output row per group via `FinalizeAgg` + `LoadGroupKey`
- ORDER BY on non-projected columns uses hidden `__sort_` columns that are
  stripped by `SortResult` after sorting.
- `CodegenError` exported for compile-time errors.
- 66 unit tests with 100% statement/line/function coverage.
