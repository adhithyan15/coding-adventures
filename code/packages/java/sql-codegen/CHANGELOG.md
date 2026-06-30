# Changelog — coding-adventures-sql-codegen

All notable changes to this package are recorded here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] — 2026-06-30

### Added
- Initial release of the Java SQL bytecode code generator (Level 1 prerequisite).
- `SqlCodegen` class with three public entry points:
  - `compile(LogicalPlan)` — optimises via `SqlOptimizer` then compiles
  - `compileOptimized(OptimizedPlan)` — compile directly from an optimised plan
  - `compileExpr(SqlExpr)` — compile a single scalar expression in isolation
- `Instruction` sealed interface with 40+ typed instruction variants:
  - Stack: `LoadConst`, `LoadColumn`, `Pop`
  - Arithmetic/logic: `BinaryOp`, `UnaryOp`, `IsNull`, `IsNotNull`
  - Predicates: `Between`, `InList`, `Like`, `CallScalar`
  - Cursor: `OpenScan`, `AdvanceCursor`, `CloseScan`
  - Row building: `BeginRow`, `EmitColumn`, `EmitRow`, `SetResultSchema`
  - Aggregates: `InitAgg`, `UpdateAgg`, `FinalizeAgg`
  - Group keys: `SaveGroupKey`, `LoadGroupKey`, `AdvanceGroupKey`
  - Post-processing: `SortResult`, `LimitResult`, `DistinctResult`
  - LEFT JOIN tracking: `JoinBeginRow`, `JoinSetMatched`, `JoinIfMatched`
  - DML: `InsertRow`, `UpdateRows`, `DeleteRows`
  - DDL: `CreateTable`, `DropTable`
  - Control flow: `Label`, `Jump`, `JumpIfFalse`, `JumpIfTrue`, `Halt`
- `Program` record: flat instruction list + label-to-index map + result schema.
- `SortKey` record for ORDER BY columns with direction and null ordering.
- Enums: `BinaryOpCode`, `UnaryOpCode`, `AggFunc`, `Direction`, `NullsOrder`.
- Two-phase aggregate compilation: accumulation loop (InitAgg/UpdateAgg) +
  group iteration loop (AdvanceGroupKey/FinalizeAgg/EmitRow).
- INNER, CROSS, and LEFT/RIGHT outer join compilation with nested-loop code generation.
- Post-processing wrapper peeling: Sort/Limit/Distinct stripped from plan top
  and appended as post-loop instructions after the scan completes.
- DML compilation: INSERT multi-row, UPDATE/DELETE with scan loop + predicate filter.
- DDL compilation: CREATE TABLE and DROP TABLE emit a single instruction + Halt.
- 36 JUnit 5 unit tests; JaCoCo line coverage ≥ 80%.
- `build.gradle.kts` with Java 21, JaCoCo 0.8.12, JUnit Jupiter 5.11.4.
- `settings.gradle.kts` declaring `includeBuild` for sql-planner and sql-optimizer.
