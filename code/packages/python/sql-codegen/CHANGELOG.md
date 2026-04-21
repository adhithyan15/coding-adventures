# Changelog

## [0.4.0] - 2026-04-21

### Added

- **`UNION` / `INTERSECT` / `EXCEPT` compilation** — set-operation plan nodes
  are now compiled end-to-end:
  - `Union(all=False)` — compiles both sides with `_compile_read`, then appends
    `DistinctResult` to deduplicate.
  - `Union(all=True)` — same but no `DistinctResult` (bag union).
  - `Intersect` — left side fills result buffer → `CaptureLeftResult` saves rows
    to `left_result` and clears the buffer → right side fills buffer →
    `IntersectResult(all)` computes the set/bag intersection.
  - `Except` — same pattern as `Intersect` but ends with `ExceptResult(all)`.

- **`INSERT … SELECT` compilation** — `Insert` nodes whose `InsertSource`
  carries a `query` sub-plan now compile to a SELECT result-capture loop
  followed by a single `InsertFromResult` instruction.  Previously this path
  raised `UnsupportedNode`.

- **Transaction IR instructions** — three new zero-field instruction types:
  - `BeginTransaction` — emitted for the `Begin` plan node.
  - `CommitTransaction` — emitted for the `Commit` plan node.
  - `RollbackTransaction` — emitted for the `Rollback` plan node.

- **New IR instructions exported from `sql_codegen`**:
  `InsertFromResult`, `CaptureLeftResult`, `IntersectResult`, `ExceptResult`,
  `BeginTransaction`, `CommitTransaction`, `RollbackTransaction`.

### Fixed

- `_compile_core` previously had no `Union` case and fell through to
  `_compile_select` → `_compile_source` → `UnsupportedNode`.  The new case
  handles `Union` directly so UNION queries compile correctly.

## [0.3.0] - 2026-04-20

### Added

- **`CallScalar(func, n_args)` instruction** — new IR instruction for scalar function
  calls.  The VM pops `n_args` arguments in push order, calls the named function from its
  scalar registry, and pushes the result.  Dispatches to `sql_vm.scalar_functions`.

- **Generic `FunctionCall` compilation** — the compiler now routes every `FunctionCall`
  AST node (including `COALESCE`) through `CallScalar` instead of special-casing it.  The
  legacy `Coalesce(n)` IR instruction is preserved for backwards compatibility.

### Changed

- `compile_expr` no longer raises `UnsupportedNode` for unknown function names.  Function
  resolution is deferred to the VM; the VM raises `UnsupportedFunction` at runtime if the
  function is not in its registry.  This makes the compilation pipeline strictly
  forward-compatible with user-defined functions registered at the VM level.

## [0.2.0] - 2026-04-19

### Added

- `AdvanceGroupKey` instruction — mirrors `AdvanceCursor` for the
  per-group emit loop. The aggregate codegen now emits this at the top
  of the emit block so the VM can iterate its internal group order and
  jump past the block when all groups have been emitted.

### Fixed

- Aggregate emit loop no longer produces an infinite `Jump(emit_start)`
  with nothing to advance the iterator. `AdvanceGroupKey(on_exhausted=…)`
  supplies the exit condition, matching the shape of `AdvanceCursor`.

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
