# Changelog — sql-codegen (Kotlin)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-06-30

### Added

- Initial implementation of `SqlCodegen` object with `compile(OptimizedPlan): Program`
  and `compileExpression(SqlExpr): List<Instruction>` public API.
- Complete `Instruction` sealed class hierarchy (36 instruction types):
  - Stack: `LoadConst`, `LoadColumn`, `LoadParam`, `LoadGroupKey`, `Pop`
  - Arithmetic: `BinaryOpInstr` (13 operators), `UnaryOpInstr` (NEG, NOT)
  - Predicates: `IsNull`, `IsNotNull`, `Between`, `Like`, `InList`
  - Scan: `OpenScan`, `AdvanceCursor`, `JumpIfExhausted`, `CloseScan`
  - Row output: `BeginRow`, `EmitColumn`, `EmitRow`
  - Aggregation: `InitAgg`, `UpdateAgg`, `FinalizeAgg`, `SaveGroupKey`, `AdvanceGroup`
  - Control flow: `Label`, `Jump`, `JumpIfTrue`, `JumpIfFalse`, `Halt`
  - DDL: `CreateTableInstr`, `DropTableInstr` (suffixed to avoid name collision with planner types)
  - DML: `InsertRow`, `UpdateRows`, `DeleteRows`
  - Transactions: `BeginTransaction`, `CommitTransaction`, `RollbackTransaction`
  - Post-ops: `SortResult`, `DistinctResult`, `LimitResult`
- `SqlValue` sealed class: `Null`, `IntVal`, `FloatVal`, `TextVal`, `BoolVal`
- `BinaryOp`, `UnaryOp`, `AggFn` enums (codegen-internal; VM does not need the planner's enums)
- `Program` data class wrapping `List<Instruction>`
- `LabelCounter` helper for unique label generation
- Post-op peeling: Sort / Limit / Distinct wrappers are peeled from the plan top and emitted
  as post-operation instructions after the core scan loop
- Two-phase aggregate compilation: accumulate (per-row UpdateAgg) + finalize (per-group FinalizeAgg)
- Cursor-loop compilation for Scan, Filter, Project, Join, Update, Delete
- Nested-loop Join compilation (INNER, CROSS; LEFT/RIGHT/FULL emit skeleton for VM to fill in NULLs)
- Union compilation (UNION ALL: two sequential loops; UNION: adds DistinctResult)
- JUnit 5 test suite with ≥50 tests covering all plan node types, all expression types,
  label naming conventions, and Jump label consistency
- JaCoCo code coverage gate enforcing ≥80% line coverage
- `settings.gradle.kts` with both `includeBuild("../sql-planner")` and
  `includeBuild("../sql-optimizer")`
- `build.gradle.kts` with `layout.buildDirectory = file("gradle-build")` to avoid
  case-insensitive filesystem collision with the `BUILD` file (lessons.md #48)
- `BUILD` and `BUILD_windows` scripts that pre-build both sibling JARs before testing
- `required_capabilities.json` declaring no external capabilities needed
- Knuth-style literate programming throughout the implementation
