# Changelog

## [0.9.0] - 2026-04-27

### Added — Phase 4a: CHECK constraints

- **`CHECK_CURSOR_ID = -1`** — sentinel cursor id used in check-expression
  instructions.  The VM temporarily maps this id to the incoming row dict so
  `LoadColumn(cursor_id=-1, column="score")` resolves to the correct value.
- **`ColumnDef.check_instrs: tuple[Instruction, ...]`** — IR ColumnDef carries the
  pre-compiled instruction sequence for its CHECK constraint; empty tuple when there
  is no constraint.
- **`compiler._to_ir_col` CHECK compilation** — when `AstColumnDef.check_expr` is
  not `None`, a fresh `_Ctx` is created with `alias_to_cursor[""] = CHECK_CURSOR_ID`
  so all unqualified column references in the expression map to the sentinel cursor;
  the compiled instructions are frozen into `check_instrs`.
- **`CHECK_CURSOR_ID` and `IrColumnDef` exported** from `sql_codegen.__init__`.

## [0.8.0] - 2026-04-27

### Added
- `AlterTable` IR instruction — holds `table: str` and `column: ColumnDef`.
- Compiler case `PlanAlterTable → AlterTable` in `_compile_plan`.
- `AlterTable` exported from `sql_codegen.__init__`.

## [0.7.0] - 2026-04-27

### Added — Phase 2: EXISTS / NOT EXISTS subquery expressions

- **`RunExistsSubquery` IR instruction** (`sql_codegen.ir`) — new instruction
  that carries a fully-resolved inner `sub_program`.  The VM executes the
  sub-program and pushes `True` if it produced at least one row, `False`
  otherwise.  Separate from `RunSubquery` so the VM can short-circuit after
  the first row without materialising the full result set.

- **`ExistsSubquery` compilation in `_compile_expr`** — when the compiler
  encounters a post-planner `ExistsSubquery(query=LogicalPlan)`, it compiles
  the inner plan to a standalone `Program` (fresh `_Ctx` so cursor/label IDs
  don't collide with the outer program) and emits a `RunExistsSubquery`
  instruction.

- **`_compile_having` accepts `ctx` parameter** — the function's `walk`
  inner closure now falls back to `_compile_expr(e, ctx)` for any expression
  not covered by the dedicated aggregate/column/literal/binary cases.  This
  enables `EXISTS (subquery)`, `NOT EXISTS`, and arbitrary boolean
  sub-expressions in `HAVING` predicates.  The call site in
  `_compile_aggregate` passes `ctx` accordingly.

- **`RunExistsSubquery` exported** — added to `sql_codegen.__init__` import
  block and `__all__`.

## [0.6.0] - 2026-04-23

### Changed — Phase 9.7: Composite (multi-column) automatic index support (IX-8)

- **`OpenIndexScan.lo / OpenIndexScan.hi` widened to `tuple[object, ...] | None`** —
  mirrors the sql-planner `IndexScan` change.  Previously stored scalar bounds
  (`object | None`); now stores tuples so that composite index bounds are
  transmitted faithfully to the VM.  The VM handler unpacks them with
  `list(ins.lo)` instead of the old `[ins.lo]` wrapping.

- **`_compile_source` pattern match updated** — the `IndexScan` destructuring
  now binds `columns=_` (was `column=_`) to stay consistent with the renamed
  field.  No semantic change; the codegen emits `OpenIndexScan` with the
  `lo`/`hi` tuples it receives from the plan node directly.

## [0.5.0] - 2026-04-21

### Added

- **`RunSubquery` IR instruction** — new instruction type for derived-table
  (subquery in FROM) execution.  Carries a `cursor_id` and a fully-resolved
  inner `sub_program`.  The VM executes the sub-program in a child state,
  materialises the result rows, and stores them under `cursor_id` so the outer
  scan loop's `AdvanceCursor` / `LoadColumn` / `CloseScan` instructions work
  transparently on the subquery rows.

- **`DerivedTable` plan-node compilation** — `_compile_source` now handles
  `DerivedTable` nodes.  The inner plan is compiled independently with its own
  cursor/label namespace (preventing ID collisions with the outer program), then
  wrapped in a `RunSubquery` instruction followed by the standard cursor loop
  (`AdvanceCursor` → body → `CloseScan`).

- **`CaseExpr` expression compilation** — `_compile_expr` now handles
  `CaseExpr` nodes by emitting a conditional-jump chain:
  ```
  compile(condition_1)
  JumpIfFalse(next_1)
  compile(result_1)
  Jump(end)
  Label(next_1)
  … (one block per WHEN branch) …
  compile(else) or LoadConst(None)
  Label(end)
  ```
  After the END label exactly one value sits on the stack — the matched
  branch result or NULL if no branch matched and there is no ELSE.

- **`RunSubquery` exported** from `sql_codegen.__init__`.

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

