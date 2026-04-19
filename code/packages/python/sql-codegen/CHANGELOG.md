# Changelog

## [0.1.0] - 2026-04-19

### Added

- Initial release. Pure `LogicalPlan` → `Program` bytecode compiler.
- Instruction set: `LoadConst`, `LoadColumn`, `BinaryOp`, `UnaryOp`,
  `IsNull`, `IsNotNull`, `Between`, `InList`, `Like`, `Coalesce`,
  `OpenScan`, `AdvanceCursor`, `CloseScan`, `BeginRow`, `EmitColumn`,
  `EmitRow`, `SetResultSchema`, `ScanAllColumns`, `InitAgg`, `UpdateAgg`,
  `FinalizeAgg`, `SaveGroupKey`, `LoadGroupKey`, `SortResult`,
  `LimitResult`, `DistinctResult`, `InsertRow`, `UpdateRows`,
  `DeleteRows`, `CreateTable`, `DropTable`, `Label`, `Jump`,
  `JumpIfFalse`, `JumpIfTrue`, `Halt`.
- `compile(plan)` entry point + single-pass label resolver.
- SELECT (Project / Filter / Scan / Join INNER+CROSS / Sort / Limit /
  Distinct), Aggregate / Having, INSERT VALUES, UPDATE, DELETE,
  CREATE TABLE, DROP TABLE, EmptyResult.
- Raises `UnsupportedNode` for LEFT / RIGHT / FULL JOIN and
  INSERT ... SELECT (deferred to v0.2).
