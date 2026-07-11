# Changelog

## [0.2.0] - Unreleased

### Fixed

- **Qualified columns across a join now resolve correctly.** `SELECT a.name,
  b.tag FROM a JOIN b ON a.id = b.aid` previously returned a single all-`NULL`
  row: a `FROM a`/`FROM b` with no `AS` alias opened both cursors under the same
  `None` key (so they collided and every `a.x`/`b.y` read whichever advanced
  last), and the projection was emitted *after* the join loop with no live
  cursor. New `compile_join_projected` keys each side by its **effective alias**
  (explicit alias, else table name — exactly what a `LoadColumn` qualifier looks
  up) and emits the projected columns *inside* the inner loop, so both the `ON`
  condition and the output columns resolve against the right row. `Project(Join)`
  now routes through this path. Verified against real SQLite by the mini-sqlite
  differential-conformance oracle (the `inner_join` case, previously a ledger
  divergence, now matches). Outer joins still degrade to a cross product (tracked
  separately); their columns now resolve correctly too.

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
