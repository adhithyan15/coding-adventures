# Changelog — java/sql-vm

## [0.1.0] — 2026-06-30

### Added

- **`SqlVm.execute(Program, Backend) → QueryResult`** — single public entry point
  that creates a fresh `VmState`, runs the dispatch loop, and returns a
  `QueryResult` record.

- **`QueryResult` record** — `List<String> columns`, `List<List<Object>> rows`,
  `int rowsAffected`.

- **Dispatch loop** over all `Instruction` variants from `SqlCodegen`:
  - Stack: `LoadConst`, `LoadColumn`, `Pop`
  - Arithmetic: `BinaryOp` (ADD/SUB/MUL/DIV/MOD/EQ/NEQ/LT/LTE/GT/GTE/AND/OR/CONCAT),
    `UnaryOp` (NEG/NOT)
  - Predicates: `IsNull`, `IsNotNull`, `Between`, `InList`, `Like`, `CallScalar`
  - Scan: `OpenScan`, `AdvanceCursor`, `CloseScan`
  - Row output: `BeginRow`, `EmitColumn`, `EmitRow`, `SetResultSchema`
  - Aggregates: `InitAgg`, `UpdateAgg`, `FinalizeAgg` (COUNT/COUNT_STAR/SUM/AVG/MIN/MAX)
  - Group keys: `SaveGroupKey`, `LoadGroupKey`, `AdvanceGroupKey`
  - Post-processing: `SortResult`, `LimitResult`, `DistinctResult`
  - JOIN tracking: `JoinBeginRow`, `JoinSetMatched`, `JoinIfMatched`
  - DML: `InsertRow`, `UpdateRows`, `DeleteRows`
  - DDL: `CreateTable`, `DropTable`
  - Control flow: `Label`, `Jump`, `JumpIfFalse`, `JumpIfTrue`, `Halt`

- **Three-valued logic**: NULL propagates through arithmetic; AND/OR have
  SQL short-circuit semantics (NULL AND FALSE → FALSE; NULL OR TRUE → TRUE).

- **Aggregates**: two-phase InitAgg → UpdateAgg × N → FinalizeAgg; NULL inputs
  skipped for all functions except COUNT(*); empty-table scalars return 0/NULL.

- **LIKE**: SQL `%`/`_` wildcards converted to Java regex; metacharacters escaped;
  NULL operands return NULL.

- **Scalar functions**: ABS, LENGTH, UPPER, LOWER, TRIM, LTRIM, RTRIM,
  SUBSTR/SUBSTRING, COALESCE, NULLIF, IFNULL/NVL, IIF, REPLACE, TYPEOF.

- **DML**: INSERT (via `InMemoryBackend.insert`), UPDATE (via positioned `Cursor`),
  DELETE (via positioned `Cursor`); `rowsAffected` counter incremented per row.

- **DDL**: CREATE TABLE (with IF NOT EXISTS), DROP TABLE (with IF EXISTS);
  SqlPlanner.ColumnDef → SqlBackend.ColumnDef conversion.

- **55 JUnit 5 tests** in `SqlVmTest`, covering all instruction categories,
  NULL semantics, three-valued logic, aggregate edge cases, and LIKE patterns.

- **JaCoCo ≥ 80% line-coverage gate** in `build.gradle.kts`.

- Knuth-style literate comments inline throughout `SqlVm.java`.
