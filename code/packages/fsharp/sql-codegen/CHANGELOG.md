# Changelog

## [0.1.0] — 2026-06-30

### Added

- Initial implementation of `SqlCodegen.compile : OptimizedPlan -> Program`
- `SqlCodegen.compileExpression : Expr -> Instruction list` helper (exported for testing)
- Full `Instruction` discriminated union with 36 instruction types mirroring the C# design:
  - Stack/memory: `LoadConst`, `LoadColumn`, `LoadParam`, `LoadGroupKey`, `LoadOuterColumn`, `Pop`
  - Arithmetic/comparison: `BinaryOpInstr`, `UnaryOpInstr`
  - Predicate tests: `IsNull`, `IsNotNull`, `Between`, `Like`, `InList`
  - Cursor control: `OpenScan`, `AdvanceCursor`, `JumpIfExhausted`, `CloseScan`
  - Row construction: `BeginRow`, `EmitColumn`, `EmitRow`
  - Aggregation: `InitAgg`, `UpdateAgg`, `FinalizeAgg`, `SaveGroupKey`, `LoadGroupKey`, `AdvanceGroup`
  - Control flow: `Label`, `Jump`, `JumpIfTrue`, `JumpIfFalse`, `Halt`
  - DDL: `CreateTable`, `DropTable`
  - DML: `InsertRow`, `UpdateRows`, `DeleteRows`
  - Transactions: `BeginTransaction`, `CommitTransaction`, `RollbackTransaction`
  - Post-processing: `SortResult`, `DistinctResult`, `LimitResult`
- Supporting operator types: `BinaryOp`, `UnaryOp`, `AggFn`
- `Program` record type: `{ Instructions: Instruction list }`
- Compilation support for:
  - SELECT with single table scan
  - SELECT with WHERE predicate (filter guard)
  - SELECT with named/aliased output columns
  - SELECT with aggregate functions (COUNT, COUNT(*), SUM, AVG, MIN, MAX)
  - SELECT with GROUP BY (SaveGroupKey / LoadGroupKey / two-phase emission)
  - SELECT with HAVING
  - SELECT with ORDER BY (SortResult post-op)
  - SELECT DISTINCT (DistinctResult post-op)
  - SELECT with LIMIT/OFFSET (LimitResult post-op)
  - SELECT with JOIN (nested loop, ON condition)
  - EmptyResult (optimizer-proven zero-row sentinel)
  - INSERT VALUES (multi-row)
  - INSERT SELECT (subquery source)
  - UPDATE with optional WHERE
  - DELETE with optional WHERE
  - CREATE TABLE / DROP TABLE (DDL)
- Label uniqueness: each scan gets a unique numeric suffix to prevent collisions in multi-scan programs
- Literate programming: all functions, types, and sections documented with explanations,
  analogies, and examples per repo CLAUDE.md requirements
- 70+ unit tests covering all query types, expression forms, instruction ordering, and label uniqueness
