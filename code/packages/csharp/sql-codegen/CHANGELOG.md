# Changelog — CodingAdventures.SqlCodegen

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] — 2026-06-30

### Added

- `SqlCodegen.cs` — single-file implementation of the bytecode code generator.
- `Program` record — compiled output containing `Instructions`, `Labels`, and `ResultSchema`.
- Full instruction hierarchy (36 instruction records):
  - Stack: `LoadConst`, `LoadColumn`, `Pop`
  - Operators: `BinaryOpInstr`, `UnaryOpInstr`, `IsNullInstr`, `IsNotNullInstr`,
    `BetweenInstr`, `InListInstr`, `LikeInstr`, `CallScalar`
  - Cursors: `OpenScan`, `AdvanceCursor`, `CloseScan`
  - Row construction: `BeginRow`, `EmitColumn`, `EmitRow`, `SetResultSchema`
  - Aggregation: `InitAgg`, `UpdateAgg`, `FinalizeAgg`, `SaveGroupKey`,
    `LoadGroupKey`, `AdvanceGroupKey`
  - Post-ops: `SortResult`, `LimitResult`, `DistinctResult`
  - Joins: `JoinBeginRow`, `JoinSetMatched`, `JoinIfMatched`
  - DML: `InsertRow`, `UpdateRows`, `DeleteRows`
  - DDL: `CreateTableInstr`, `DropTableInstr`
  - Control flow: `CodegenLabel`, `Jump`, `JumpIfFalse`, `JumpIfTrue`, `Halt`
- `SqlCodegen` static class with three entry points:
  - `Compile(LogicalPlan)` — convenience overload, lifts and optimizes before compiling
  - `CompileOptimized(OptimizedPlan)` — main entry point
  - `CompileExpr(SqlExpr)` — isolated expression compilation for tests
- Compilation strategy:
  - Wrapper peeling for Sort/Limit/Distinct (emitted as post-ops after the scan body)
  - Nested-loop scan bodies for Scan, Filter, INNER/CROSS JOIN, LEFT JOIN
  - Two-phase aggregation (accumulation scan + group emit loop)
  - DML: INSERT per-row loops, UPDATE/DELETE cursor-scan loops with optional predicate
  - DDL: direct instruction emit
  - Label resolution pass at end of compilation
- 36 xUnit tests covering all plan node types and expression subtypes.
- `coverlet.msbuild` integration with ≥80% line coverage threshold.

### Notes

- `BinaryOpInstr` and `UnaryOpInstr` are deliberately renamed from the spec's
  `BinaryOp`/`UnaryOp` to avoid ambiguity with identically-named `SqlExpr` subtypes
  in the planner namespace.  The VM should use these instruction record names.
- The planner's `AggFunction.Count` maps to `AggFunc.Count`; `COUNT(*)` uses
  `AggFunc.Count` with a `LoadConst(null)` argument (the `AggArg.Star` case).
- `CountStar` is defined in the `AggFunc` enum for VM use but the codegen always
  maps `AggArg.Star` to `AggFunc.Count` to keep the count/countstar distinction
  inside the VM where it belongs.
