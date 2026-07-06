# Changelog - @coding-adventures/sql-vm

## [0.1.0] - 2026-07-06

### Added
- Initial TypeScript implementation of the SQL virtual machine.
- `execute(program: Program, db: Database): QueryResult` — dispatch loop that
  runs a compiled IR `Program` against an in-memory table store.
- `Database` type: `Map<string, TableData>` where `TableData = { columns: string[], rows: Record<string, SqlValue>[] }`.
- `QueryResult` type: `{ columns: string[], rows: SqlValue[][], rowsAffected: number }`.
  `rowsAffected` is `-1` for SELECT and DDL; the affected-row count for DML.
- `VmError` class for runtime errors (missing table, stack underflow,
  unknown instruction, unknown label, unknown function).
- Full dispatch loop for all IR instructions:
  - Stack: `LoadConst`, `LoadNull`, `LoadColumn` (cursorId `-2` = aggBuffer,
    `-1` = auto-resolve cursors then rowBuffer, `≥0` = specific cursor)
  - Arithmetic / logic: `BinaryOp` (+,-,*,/,%,||,=,<>,<,<=,>,>=,AND,OR),
    `UnaryOp` (-,+,NOT), `IsNullInstr`, `IsNotNullInstr`, `BetweenInstr`,
    `LikeInstr`, `InList`, `Coalesce`
  - Built-in scalar functions via `CallFunc`: UPPER, LOWER, LENGTH, ABS,
    ROUND, SUBSTR/SUBSTRING, TRIM, LTRIM, RTRIM, REPLACE, COALESCE, NULLIF,
    IFNULL, IIF, TYPEOF
  - Table scan: `OpenScan`, `AdvanceCursor`, `JumpIfExhausted`, `CloseScan`
  - Row output: `BeginRow`, `EmitColumn` (with `__star__` expand-all),
    `EmitRow`, `SetResultSchema`
  - Aggregates: `InitAgg`, `UpdateAgg`, `FinalizeAgg`, `SaveGroupKey`,
    `LoadGroupKey`, `AdvanceGroup`, `JumpIfGroupsDone`
    - Aggregate functions: COUNT, SUM, MIN, MAX, AVG, GROUP_CONCAT
  - Post-processing: `SortResult` (with `nullsLast` per key + `stripPrefix`),
    `DistinctResult`, `LimitResult`
  - DML: `InsertRow`, `UpdateRows`, `DeleteRows`
  - DDL: `CreateTable` (with `ifNotExists`), `DropTable` (with `ifExists`)
  - Transactions: `BeginTransaction`, `CommitTransaction`, `RollbackTransaction`
    (no-ops in the in-memory engine)
  - Control flow: `Label`, `Jump`, `JumpIfTrue`, `JumpIfFalse`, `Halt`
- `__dual__` virtual single-row table for FROM-less SELECT.
- NULL propagation for all arithmetic and comparison operators.
- SQL three-valued logic (TRUE/FALSE/NULL) for AND and OR.
- 121 unit tests with 97.6% statement/100% function/99.5% line coverage.
