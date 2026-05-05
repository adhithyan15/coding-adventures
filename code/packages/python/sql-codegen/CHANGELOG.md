# Changelog

## [1.17.0] - 2026-05-05

### Added

- **`UpsertAssignment`, `UpsertSpec`, `LoadExcludedColumn` IR nodes** (`ir.py`) —
  three new frozen dataclasses completing the upsert IR layer:
  - `UpsertAssignment(column: str, instructions: tuple[Instruction, ...])` — one
    SET column with its pre-compiled expression as a self-contained instruction
    sequence evaluated by `_upsert_apply` in the VM.
  - `UpsertSpec(conflict_target, do_nothing, assignments)` — the compiled upsert
    clause carried on `InsertRow.upsert` and `InsertFromResult.upsert`.
  - `LoadExcludedColumn(col: str)` — pushes the named column's value from the
    *would-be-inserted* row onto the operand stack.  The VM resolves this against
    `_VmState.excluded_row` which is populated by `_upsert_apply` before evaluating
    each SET expression.

- **`upsert: UpsertSpec | None` on `InsertRow` and `InsertFromResult`** (`ir.py`) —
  both insert IR nodes now carry the optional upsert spec.  When `None`, the VM
  behaves as before (no ON CONFLICT handling beyond `on_conflict`).

- **`_compile_upsert(upsert, ctx)` in `compiler.py`** — compiles a
  `PlanUpsertAction` into a `UpsertSpec`.  For DO-NOTHING returns an assignment-free
  `UpsertSpec(do_nothing=True)`.  For DO-UPDATE compiles each assignment's `Expr`
  into a flat instruction tuple via `_compile_expr`, which handles `ExcludedColumn`
  as `LoadExcludedColumn`.

- **`ExcludedColumn` case in `_compile_expr`** — `ExcludedColumn(col=c)` compiles
  to `[LoadExcludedColumn(col=c)]`.

- **New exports in `__init__.py`**: `LoadExcludedColumn`, `UpsertSpec`,
  `IrUpsertAssignment`.

## [1.16.0] - 2026-05-05

### Added

- **`NO_COLUMN_DEFAULT` sentinel in `ir.py`** — module-level singleton
  (`_NoColumnDefault` / `NO_COLUMN_DEFAULT: Final`) that distinguishes "no
  DEFAULT clause" from "DEFAULT NULL" in the IR `ColumnDef`.  Decoupled from
  `sql_backend.schema.NO_DEFAULT` so the IR layer does not import from the
  backend.

- **`default: object = NO_COLUMN_DEFAULT` on IR `ColumnDef`** — the column
  definition dataclass now carries the declared DEFAULT literal value (an
  integer, float, string, or `None` for DEFAULT NULL) through to the VM.
  When no DEFAULT clause is present the field holds the `NO_COLUMN_DEFAULT`
  sentinel and the VM passes `NO_DEFAULT` to the backend (preserving existing
  behaviour).

- **`_to_ir_col` passes `default=ir_default`** (`compiler.py`) — converts the
  backend's `NO_DEFAULT` sentinel to the IR's `NO_COLUMN_DEFAULT`, then stores
  any other value as-is.  Imports `_BACKEND_NO_DEFAULT` from
  `sql_backend.schema` and `NO_COLUMN_DEFAULT` from the local `ir` module for
  the conversion.

## [1.15.0] - 2026-05-04

### Added

- **`on_conflict` field on `InsertRow` and `InsertFromResult`** (`ir.py`) —
  both INSERT IR instructions now carry `on_conflict: str | None` matching the
  planner's `Insert.on_conflict`.  The VM reads this field to choose between
  REPLACE (pre-delete conflicts), IGNORE (swallow `ConstraintViolation`), and
  default ABORT behaviour.

- **`unique` field on IR `ColumnDef`** (`ir.py`) — the `ColumnDef` dataclass
  now includes `unique: bool = False` so the UNIQUE column constraint is no
  longer silently dropped when converting from the planner's backend-schema
  `ColumnDef` to IR.  Without this field the VM was creating tables whose
  non-PK UNIQUE columns had `unique=False` in the backend, making duplicate
  values pass silently.

- **`_to_ir_col` now passes `unique=c.unique`** (`compiler.py`) — ensures the
  UNIQUE flag flows from `AstColumnDef` (which is `sql_backend.schema.ColumnDef`)
  through IR all the way to the VM.

- **Codegen passes `on_conflict`** — `InsertRow(...)` and `InsertFromResult(...)`
  constructions now include `on_conflict=ins.on_conflict`.

## [1.14.0] - 2026-05-04

### Added

- **`BinaryOpCode.CONCAT` mapping** (`compiler.py :: _BINOP_MAP`) — the
  planner's `BinaryOp.CONCAT` (`||`) now maps to `BinaryOpCode.CONCAT`,
  which the VM executes via its `_concat` kernel.  Previously missing, this
  caused the codegen to raise `KeyError` for `||` expressions involving
  non-constant operands.

### Tests

- `tests/test_expressions.py` — added `test_concat_emits_binary_concat` and
  `test_concat_column_and_literal` to verify that `BinaryOp.CONCAT` compiles
  to the correct `BinaryOp(op=BinaryOpCode.CONCAT)` instruction with
  post-order operand push.

## [1.13.0] - 2026-05-04

### Added

- **`SingleRow` compilation** (`compiler.py`) — `_compile_source` now handles
  `SingleRow()` plan nodes by invoking `body(ctx)` exactly once with no
  cursor machinery.  This powers SELECT without FROM (e.g. `SELECT 1 + 1`).

## [1.12.0] - 2026-05-04

### Added

- **`GROUP_CONCAT` in `AggFunc` enum** (`ir.py`) — new value
  `GROUP_CONCAT = "GROUP_CONCAT"` added to the IR aggregate-function enum.
- **`separator` field on `InitAgg`** (`ir.py`) — `str` field defaulting to
  `","`.  Baked in at compile time from the SQL literal; ignored for all
  functions except `GROUP_CONCAT`.
- **`func` and `separator` fields on `FinalizeAgg`** (`ir.py`) — carry the
  aggregate function kind and GROUP_CONCAT separator as fallback values for
  the empty-table implicit-single-group case.  Both have defaults
  (`COUNT_STAR` / `","`) for backward compatibility with existing
  `FinalizeAgg(slot=…)` call sites.
- **`has_group_by` field on `AdvanceGroupKey`** (`ir.py`) — `bool` field
  defaulting to `True`.  When `False` the VM synthesises an implicit group
  for no-GROUP-BY queries over empty tables so that exactly one result row
  is emitted, matching the SQL standard.
- **`GROUP_CONCAT` codegen** (`compiler.py`) — `_plan_agg_to_ir` now maps
  `AggFunc.GROUP_CONCAT` → `IrAggFunc.GROUP_CONCAT`.  The compiler wires
  the `separator` (from `AggregateItem`) into `InitAgg` and passes
  `func`/`separator` through to every `FinalizeAgg` emission (main emit
  loop and HAVING predicate).
- **Compile-time integer enforcement** (`compiler.py`) — `_literal_val`
  rejects non-integer literals for `LAG/LEAD offset`, `NTILE n`, and
  `NTH_VALUE n` with a descriptive `UnsupportedNode`.

### Changed

- `AdvanceGroupKey(on_exhausted=…)` emissions now pass
  `has_group_by=bool(group_by)` so the VM can distinguish implicit-single-
  group from multi-group aggregates.
- `FinalizeAgg(slot=…)` emissions now always carry `func` and `separator`
  matching the corresponding aggregate; the VM uses these for lazy slot
  initialisation when `InitAgg` was never reached (empty-table path).

## [1.11.0] - 2026-05-04

### Added

- **New `WinFunc` enum values** (`ir.py`) — `LAG`, `LEAD`, `NTH_VALUE`,
  `NTILE`, `PERCENT_RANK`, `CUME_DIST` added to the `WinFunc` enumeration.
- **`extra_args` field on `WinFuncSpec`** (`ir.py`) — `tuple[object, ...]`
  that carries literal constants for multi-argument window functions:
  - `LAG` / `LEAD` → `(offset: int, default: SqlValue)` (always 2 elements)
  - `NTILE` → `(n: int,)` — the bucket count
  - `NTH_VALUE` → `(n: int,)` — the 1-indexed row position
  - `PERCENT_RANK`, `CUME_DIST` → `()` (empty, no extra args needed)
- **Extended `_WIN_FUNC_MAP`** (`compiler.py`) — the mapping now includes
  all six new functions.
- **Rewritten `_to_ir_win_spec`** (`compiler.py`) — converts planner-level
  `WindowFuncSpec` to IR `WinFuncSpec`, handling each function's unique
  argument shape:
  - `LAG`/`LEAD` — normalises `extra_args` to exactly `(offset, default)`,
    defaulting to `(1, None)` when arguments are omitted.
  - `NTILE` — the literal bucket count in `arg_expr` is moved to
    `extra_args` and `arg_col` is set to `None` (NTILE has no column arg).
  - `NTH_VALUE` — column is `arg_col`; `n` is `extra_args[0]`.
  - `PERCENT_RANK`, `CUME_DIST` — no `arg_col` or `extra_args`.
  - Negated-literal folding: `UnaryExpr(NEG, Literal(n))` (produced by
    the parser for `-1`, `-2`, …) is constant-folded to `-n` inside
    `_literal_val` so that `LAG(col, 1, -1)` works correctly.

## [1.10.0] - 2026-05-04

### Added

- **`LoadLastInsertedColumn(col)` IR instruction** (`ir.py`) — pushes the
  value of `col` from the most recently inserted row onto the value stack.
  Used by INSERT … RETURNING where there is no open cursor after the insert
  completes.  Exported from `sql_codegen.__init__`.
- **RETURNING clause compilation** (`compiler.py`) — the compiler now handles
  the `returning` field on `Insert`, `Update`, and `Delete` plan nodes:
  - `Insert RETURNING` — emits `SetResultSchema` at the top; after each
    `InsertRow` emits `BeginRow` + `LoadLastInsertedColumn`/`EmitColumn` per
    column + `EmitRow`.
  - `Update RETURNING` — emits `SetResultSchema` at the top; after
    `UpdateRows` emits `BeginRow` + `LoadColumn`/`EmitColumn` per column +
    `EmitRow` (reads the *post-update* row).
  - `Delete RETURNING` — emits `SetResultSchema` at the top; *before*
    `DeleteRows` emits `BeginRow` + `LoadColumn`/`EmitColumn` per column +
    `EmitRow` (captures the row *before* deletion).

## [1.9.0] - 2026-05-04

### Added

- **`LoadOuterColumn(cursor_id, col)` IR instruction** (`ir.py`) — pushes
  the value of `col` from the outer query's cursor snapshot onto the value
  stack.  Used by correlated subqueries where an inner program needs to read
  a column from the enclosing query's current row.  Returns `None` (NULL) if
  the cursor ID is absent from the snapshot or the column is not present.
- **`outer_alias_to_cursor` field on `_Ctx`** (`compiler.py`) — holds the
  outer query's `alias → cursor_id` mapping.  Populated when compiling
  subprograms for `RunExistsSubquery`, `RunScalarSubquery`, and `RunInSubquery`
  by copying `ctx.alias_to_cursor` from the enclosing context.
- **`CorrelatedRef` compilation** (`compiler.py`) — `_compile_expr` handles
  `CorrelatedRef(outer_alias, col)` by looking up `outer_alias` in
  `ctx.outer_alias_to_cursor` and emitting `LoadOuterColumn(cursor_id, col)`.
  Raises `UnsupportedNode` if `outer_alias_to_cursor` is absent (top-level
  query — should never happen in well-formed plans).
- **`LoadOuterColumn`** exported from `sql_codegen.__init__`.

## [1.8.0] - 2026-05-04

### Added

- **`RunInSubquery` IR instruction** (`ir.py`) — stack instruction that
  pops the test value, executes an embedded `sub_program`, materializes
  the first column of every result row into a set, then pushes the
  membership result.  The `negate: bool` field inverts the result for
  `NOT IN`.  SQL three-valued logic is preserved: if the test value is
  `NULL`, pushes `None`; if the set contains `NULL` and the value was
  not found, pushes `None` (UNKNOWN); otherwise pushes `True`/`False`.
- **`InSubquery` / `NotInSubquery` compilation** (`compiler.py`) —
  `_compile_expr` handles both new planner expression nodes by compiling
  the inner plan into an embedded `Program` (same lifecycle as
  `RunScalarSubquery` and `RunExistsSubquery`) and emitting
  `RunInSubquery(sub_program=..., negate=...)`.
- **HAVING `_compile_having` fix** (`compiler.py`) — the aggregate-slot
  lookup condition now accepts both `arg=None` (legacy direct-
  construction form for `COUNT(*)`) and `arg=FuncArg(star=True)` (the
  form produced by the planner), fixing `COUNT(*) > N` in HAVING when
  used inside an `IN` subquery.
- **`RunInSubquery`** exported from `sql_codegen.__init__`.

## [1.7.0] - 2026-05-04

### Added

- **FULL [OUTER] JOIN compilation** — `_compile_join` now handles
  `JoinKind.FULL` via a two-pass strategy:
  - **Pass 1** runs `_compile_join(lft, rgt, LEFT, ...)` — emits every
    left row (with NULL right columns when no right row matched).
  - **Pass 2** runs a right-anti-join: scans `rgt` as the outer loop,
    `lft` as the inner (mark-only) loop.  `foj_anti_inner` checks the ON
    condition and calls `JoinSetMatched` but does **not** call `body(c)`.
    After the inner scan, `JoinIfMatched` jumps past the body if any left
    row matched (those rows were already emitted in Pass 1).  On the
    fall-through path the left cursor is closed, so `LoadColumn` for
    left-side columns returns `NULL`.
  - Cursor IDs across the two passes are unique: Pass 1 allocates IDs 0
    and 1; Pass 2 calls `_compile_source` again, bumping the counter to
    IDs 2 and 3.  When `body(c)` is invoked in Pass 2 the
    `alias_to_cursor` map has already been updated to the Pass 2 IDs, so
    `LoadColumn` instructions generated by `body` reference the correct
    cursors.
- **`test_full_join_compiles`** replaces `test_full_join_raises` in
  `tests/test_select.py`; asserts four `OpenScan` / `CloseScan` total
  (two per pass) and the presence of all three outer-join instructions.

## [1.6.0] - 2026-05-04

### Added

- **RIGHT [OUTER] JOIN compilation** — `_compile_join` now handles
  `JoinKind.RIGHT` by swapping `lft`/`rgt` and delegating to the LEFT
  JOIN path. The ON condition and body function both reference columns
  by table alias (via `alias_to_cursor`), so reversing execution order
  is sufficient: the original right table becomes the outer "left"
  (preserved for every row) and the original left table becomes the
  inner "right" (null-padded when no ON match is found).
- **`test_right_join_compiles`** replaces `test_right_join_raises` in
  `tests/test_select.py`; a new `test_full_join_raises` confirms FULL
  JOIN still raises `UnsupportedNode`.

## [1.5.0] - 2026-05-04

### Added

- **LEFT [OUTER] JOIN compilation** — `_compile_join` now handles
  `JoinKind.LEFT`, emitting a nested-loop outer join using three new
  match-tracking IR instructions (`JoinBeginRow`, `JoinSetMatched`,
  `JoinIfMatched`). When the right scan exhausts without a match the
  right cursor is closed; any subsequent `LoadColumn` for right-side
  columns returns `NULL` automatically, providing null-padding without
  a dedicated `NullRow` instruction.
- **`JoinBeginRow` IR instruction** — pushes `False` onto the VM's
  `join_match_stack` at the start of each left row.
- **`JoinSetMatched` IR instruction** — sets `join_match_stack[-1] = True`
  when an ON-condition match is found.
- **`JoinIfMatched(label)` IR instruction** — pops the match stack; if
  `True` jumps to *label* (skipping the null-padded emission path).
- All three exported from `sql_codegen.__init__` and added to the
  `Instruction` type union in `ir.py`.

### Fixed

- **Duplicate label bug in `_compile_select` body closure** — the
  `filter_skip` label was previously generated once outside the body
  closure. Calling `body(c)` twice (matched path + null-padded path in
  a LEFT JOIN) produced two `Label("filter_skip_N")` entries with
  identical names, causing `_resolve_labels` to overwrite the first jump
  target. The label is now generated *inside* the body closure so each
  invocation gets a unique name.

## [1.4.0] - 2026-04-28

### Added

- **`RunScalarSubquery(sub_program)` IR instruction** — compiles a
  `ScalarSubquery` plan expression into an embedded sub-program that is
  executed at runtime; the VM pushes the single result value or NULL.
- **`primary_key: bool` on `ColumnDef`** — IR column definition now carries
  the primary key flag so the VM can pass it to the backend on `CREATE TABLE`.

### Fixed

- **`_compile_core` handles `Project(Aggregate)`** — scalar subquery inner
  plans contain an unflattenened `Project(Aggregate(...))` shape. Added two
  match cases before the default fall-through so aggregate sub-programs
  compile correctly without requiring `_flatten_project_over_aggregate`.

## [1.3.0] - 2026-04-28

### Added — Phase 9: SQL Triggers

- **`CreateTriggerDef` IR instruction** — carries `name`, `timing`, `event`,
  `table`, `body_sql`; emitted by the compiler for `CreateTrigger` plan nodes.
- **`DropTriggerDef` IR instruction** — carries `name`, `if_exists`; emitted
  for `DropTrigger` plan nodes.
- Both exported from `sql_codegen.__init__` and added to the `Instruction`
  type union in `ir.py`.

## [1.2.0] - 2026-04-27

### Added — Phase 8: Window Functions (OVER / PARTITION BY)

- **`WinFunc` enum** — `ROW_NUMBER`, `RANK`, `DENSE_RANK`, `SUM`, `COUNT`,
  `COUNT_STAR`, `AVG`, `MIN`, `MAX`, `FIRST_VALUE`, `LAST_VALUE`.
- **`WinFuncSpec` IR dataclass** — `func: WinFunc`, `arg_col: str | None`,
  `partition_cols`, `order_cols`, `result_col`.
- **`ComputeWindowFunctions` instruction** — post-processing instruction
  (analogous to `SortResult` / `LimitResult`) that runs after all rows are
  materialised.  Carries `specs` and `output_cols`.
- **`_compile_plan` special case for `WindowAgg`** — emits
  `SetResultSchema(inner_schema) + inner_instrs + ComputeWindowFunctions`.
  Prepending `SetResultSchema(inner_schema)` is critical: it ensures
  `result.columns` reflects the inner column layout (not the outer
  `output_cols`) when `ComputeWindowFunctions` looks up arg/partition/order
  columns by name.
- **`_compile_core` case for `WindowAgg`** — same logic for when `WindowAgg`
  is wrapped inside `Sort` / `Limit` / `Distinct`.
- **`_schema_of` case for `WindowAgg`** — returns `output_cols`.
- **`_to_ir_win_spec()` helper** — maps `PlanWindowFuncSpec` → `WinFuncSpec`,
  resolving `arg_expr` to a column name in the inner schema.
- All new types exported via `__all__`.

## [1.1.0] - 2026-04-27

### Added — Phase 5b: Recursive CTEs

- **`OpenWorkingSetScan` IR instruction** — opens a fresh cursor over the
  current working-set rows stored in `_VmState.working_set_data`.  Emitted
  at the top of each `WorkingSetScan` loop so that self-references inside a
  JOIN (which close and reopen the inner cursor on each outer iteration) work
  correctly without exhausting the cursor.
- **`RunRecursiveCTE` IR instruction** — drives the fixed-point iteration:
  runs `anchor_program` once, then runs `recursive_program` against the
  previous working set until the recursive step produces zero new rows.
  Carries `cursor_id`, `anchor_program`, `recursive_program`,
  `working_cursor_id`, and `union_all` flag.
- **`WorkingSetScan` compiler case** in `_compile_source` — emits
  `OpenWorkingSetScan` + loop scaffolding (identical shape to `Scan` / derived
  table, but opening from the VM's working set rather than a backend table).
- **`RecursiveCTE` compiler case** in `_compile_source` — compiles anchor and
  recursive sub-programs in isolated `_Ctx` instances (recursive ctx reserves
  cursor 0 for the working set), resolves labels, wraps both as `Program`
  objects, emits `RunRecursiveCTE`, and adds the outer advance/loop/close
  scaffolding for the caller to iterate results.
- **`_Ctx.working_set_cursor_id`** — optional int used by `WorkingSetScan` to
  know which cursor id to emit `OpenWorkingSetScan` for (defaults to 0 in the
  recursive sub-program context).
- Both new IR instructions exported from `sql_codegen.__init__`.

## [1.0.0] - 2026-04-27

### Added — Phase 4b: FOREIGN KEY constraints

- **`IrColumnDef.foreign_key: tuple[str, str | None] | None`** — carries the
  `(ref_table, ref_col_or_None)` FK reference into the VM.
- **`_to_ir_col()` passes `foreign_key` through** — reads `c.foreign_key` from
  the AST/backend `ColumnDef` and copies it into the IR struct.

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

