# Changelog

## [0.1.0] - 2026-07-01

### Added

- Initial implementation of the Rust bytecode code generator for the Mini-SQLite Level 1 pipeline.
- `compile(plan: &OptimizedPlan) -> Program` public API function.
- `Program` struct containing a flat `Vec<Instruction>`.
- Complete `Instruction` enum with 37 variants covering:
  - Stack ops: `LoadConst`, `LoadColumn`
  - Arithmetic/logic: `BinaryOpInstr`, `UnaryOpInstr`
  - Null tests: `IsNull`, `IsNotNull`
  - Predicates: `Between`, `Like`, `InList`
  - Scan control: `OpenScan`, `AdvanceCursor`, `JumpIfExhausted`, `CloseScan`
  - Row assembly: `BeginRow`, `EmitColumn`, `EmitRow`
  - Aggregation: `InitAgg`, `UpdateAgg`, `FinalizeAgg`, `SaveGroupKey`
  - Control flow: `Label`, `Jump`, `JumpIfTrue`, `JumpIfFalse`, `Halt`
  - DDL: `CreateTableInstr`, `DropTableInstr`
  - DML: `InsertRow`, `UpdateRows`, `DeleteRows`
  - Transactions: `BeginTransaction`, `CommitTransaction`, `RollbackTransaction`
  - Post-processing: `SortResult`, `DistinctResult`, `LimitResult`
- `BinaryOp`, `UnaryOp`, `AggFn`, `CompiledSortKey` supporting types.
- Post-op peeling: `Sort`, `Limit`, `Distinct` wrappers are stripped and emitted after `Halt`.
- Scan loop pattern: `OpenScan` → `Label` → `AdvanceCursor` → `JumpIfExhausted` → body → `Jump` → `Label` → `CloseScan`.
- Filter compilation: predicate check with `JumpIfFalse` inside the scan loop.
- Project compilation: `BeginRow` + per-column expression + `EmitColumn` + `EmitRow`.
- Aggregate compilation: two-phase (accumulation loop + finalization/emission).
- Having compilation: predicate check after FinalizeAgg, before EmitRow.
- Join compilation: nested-loop join with optional ON condition.
- INSERT compilation: one `InsertRow` per VALUES row.
- UPDATE compilation: scan loop with optional predicate filter + `UpdateRows`.
- DELETE compilation: scan loop with optional predicate filter + `DeleteRows`.
- DDL compilation: `CreateTableInstr` and `DropTableInstr` as single-instruction programs.
- Thread-local recursion depth guard (`MAX_EXPR_DEPTH = 512`) to prevent stack overflow.
- 50+ unit tests covering all major compilation paths.
- Knuth-style literate programming: all code includes inline explanations, diagrams, and analogies.
