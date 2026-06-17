# Changelog

## [1.43.0] - 2026-06-16

### Fixed

- **Literal expressions now get SQLite-compatible column display names** —
  `_column_display_name()` previously returned `None` for `Literal` nodes,
  causing `_projection_name()` to fall back to `"?"` for every unnamed
  constant expression.  When a single ``SELECT`` had two or more literal
  columns (e.g. ``SELECT 1, 2``), both columns received the key ``"?"``,
  and the VM's ``_do_run_subquery()`` converted rows to dicts via
  ``dict(zip(cols, row))``, silently losing all but the last value.

  Fix: ``_column_display_name`` now handles ``Literal`` by returning the
  surface representation SQLite uses:

  | Literal type | Display name example  |
  |--------------|-----------------------|
  | `int`        | `"1"`, `"42"`         |
  | `float`      | `"1.5"`, `"3.14"`     |
  | `str`        | `"'hello'"`           |
  | `bytes`      | `"X'DEADBEEF'"`       |
  | `None`       | `"NULL"`              |

  This makes ``SELECT 1, 2`` produce column names ``("1", "2")`` instead
  of ``("?", "?")``, fixing ``SELECT * FROM (SELECT 1, 2)`` which
  previously returned ``(2,)`` instead of the correct ``(1, 2)``.

- **RowIdRef now covered in ``_column_display_name``** — added unit tests
  that exercise both the ``RowIdRef → "rowid"`` branch and the new
  ``Literal`` branch, bringing overall package coverage to **80.48 %**.

## [1.42.0] - 2026-06-16

### Fixed

- **Hidden-column injection now covers `PlanWindowAgg`** — ``ORDER BY``
  expressions referencing columns absent from ``output_cols`` no longer
  crash with ``ValueError: tuple.index(x): x not in tuple`` when the
  inner plan is a window aggregation node.

  Previously the hidden-column injection pass in ``_compile_read``
  only activated for ``Project`` inner nodes.  When the inner node was
  a ``PlanWindowAgg`` (e.g. ``SELECT grp, SUM(val) OVER (PARTITION BY
  grp) … ORDER BY grp, val``), ``ComputeWindowFunctions`` had already
  projected away ``val`` before ``SortResult`` tried to look it up.

  Fix: a new ``elif isinstance(cur, PlanWindowAgg)`` branch extends
  ``output_cols`` with the missing sort-key columns as hidden trailing
  entries.  ``ComputeWindowFunctions`` passes them through; a
  ``StripTrailingColumns`` instruction inserted right after
  ``SortResult`` removes them so callers see only the original SELECT
  columns.  No ``extended_schema`` override is needed — the window
  codegen path manages its own ``SetResultSchema`` lifecycle.

## [1.41.0] - 2026-05-24

### Added

- ``ColumnDef.check_expr_text: str`` — passes the source text of a
  CHECK predicate through the IR so the VM can quote it verbatim in
  ``CHECK constraint failed: <expr_text>`` error messages (matches
  SQLite).  ``_to_ir_col`` reads the field off the planner-level
  ``ColumnDef`` (which the adapter populates).  Defaults to ``""``
  for back-compat — the VM falls back to the older ``<table>.<col>``
  form when the text is unavailable.

## [1.40.0] - 2026-05-23

### Fixed

- ``ORDER BY <expr>`` with arbitrary expressions (``a+b``,
  ``UPPER(name)``, ``CASE WHEN … END``, …) no longer raises
  ``ValueError: tuple.index(x): x not in tuple`` at SortResult time.

  The hidden-column injection pass in ``_compile_read`` previously
  short-circuited when the planner sort key's display name was ``"?"``
  (the fallback for un-named expressions), so the VM ended up looking
  up a non-existent ``"?"`` column in the result schema.  The pass now
  recognises the expression case: each ``"?"``-named sort key is
  projected as a hidden trailing column under a synthetic per-position
  name (``__sortkey_0``, ``__sortkey_1``, …), and the corresponding
  SortKey IR is rewritten to look up that synthetic name.
  ``StripTrailingColumns`` removes the extras after the sort, so the
  output shape is unchanged for the caller.

  Position-local synthetic names are required because two ``ORDER BY``
  terms with the same ``"?"`` display name would otherwise collide on
  one hidden slot and silently sort by only the first expression.

## [1.39.0] - 2026-05-23

### Changed

- ``_to_ir_col`` now uses the raw ``ColumnDef.not_null`` flag for the
  IR's ``nullable`` field (was ``c.effective_not_null()`` which folded
  in PRIMARY KEY's implicit NOT NULL).  Constraint enforcement at the
  backend is unchanged — the backend's ``effective_not_null()``
  reapplies the PK check at insert/update time.  Required for
  ``PRAGMA table_info`` to distinguish explicit-vs-implicit NOT NULL
  (matches sqlite3).

## [1.38.0] - 2026-05-23

### Added

- IR ``ColumnDef`` gains an ``autoincrement: bool = False`` field
  carrying SQLite's ``AUTOINCREMENT`` clause through to the VM.
  The compiler forwards ``c.autoincrement`` from the planner-layer
  ColumnDef unchanged.

## [1.37.0] - 2026-05-23

### Added

- IR ``CreateTable`` gained a ``strict: bool = False`` field; the
  compiler forwards ``PlanCreateTable.strict`` through unchanged so the
  VM can forward it to ``Backend.create_table(strict=...)``.

## [1.36.0] - 2026-05-23

### Added

- IR ``AlterTable`` gained three new optional fields —
  ``rename_to``, ``rename_column``, ``drop_column`` — mirroring the
  planner side.  The compiler dispatches on the plan node and
  forwards whichever is set to the IR node.

## [1.35.0] - 2026-05-23

### Added

- ``ColumnDef.collation: str | None`` IR field — carries the column's
  declared COLLATE clause from CREATE TABLE through to the VM.  The
  codegen's column-def builder forwards
  ``planner_col.collation → ir_col.collation`` unchanged.

## [1.34.0] - 2026-05-22

### Added

- ``SortKey.collation: str | None`` IR field, mirroring the planner
  side.  ``_to_ir_sort_key`` forwards the collation name from the
  plan SortKey to the IR SortKey unchanged.  ``None`` means BINARY
  (the default).

## [1.33.0] - 2026-05-21

### Added

- New IR opcodes for the bitwise operators introduced in sql-planner
  0.36 and sql-vm 1.49:
  - `BinaryOpCode.BIT_AND`, `BinaryOpCode.BIT_OR`,
    `BinaryOpCode.BIT_SHL`, `BinaryOpCode.BIT_SHR`.
  - `UnaryOpCode.BIT_NOT`.
- Compiler mappings from `BinaryOp.BIT_*` and `UnaryOp.BIT_NOT` (planner
  AST) to the matching IR opcodes, so codegen produces the right VM
  instructions for `a & b`, `a | b`, `a << b`, `a >> b`, and `~a` with
  no plan-level changes required by callers.

## [1.32.0] - 2026-05-19

### Fixed

- **RIGHT JOIN + ``SELECT *`` column order**.  The codegen implements
  RIGHT JOIN by swapping the two sides and emitting a LEFT JOIN
  (``_compile_join(rgt, lft, JoinKind.LEFT, …)``), which works for
  explicit column projections (the Project node above the join controls
  output order by name).  But ``SELECT *`` iterates
  ``ctx.alias_to_cursor`` in insertion order — and the swap caused the
  right-side cursor to be allocated first, emitting RIGHT columns
  before LEFT columns and diverging from SQLite.

  The RIGHT JOIN branch now wraps ``body`` in a closure that reorders
  ``alias_to_cursor`` (original-left first, original-right second) for
  the duration of each body invocation, restoring left→right column
  order in ``SELECT *`` output.

  Example::

      Before: SELECT * FROM a RIGHT JOIN b ON a.id = b.id
              → (b.id, b.y, a.id, a.x)   -- wrong order
      Now:    → (a.id, a.x, b.id, b.y)   -- matches sqlite3

- **New helper ``_plan_alias``** — mirrors the alias-extraction logic
  in ``_compile_source`` for each plan-node type.  Used by the RIGHT
  JOIN reorder to find the lft/rgt aliases without duplicating the
  match.

## [1.31.0] - 2026-05-19

### Fixed

- **``SELECT *`` now emits columns from every open cursor** in
  ``_compile_project_body``.  The Wildcard branch previously called
  ``_primary_cursor`` and emitted ``ScanAllColumns`` for *only* the
  first scan opened, silently dropping columns from any additional
  cross-join sources.  Now iterates ``ctx.alias_to_cursor.values()``
  in insertion order, emitting one ``ScanAllColumns`` per cursor.

  Affected queries:

      SELECT * FROM a, b                         -- two plain tables
      SELECT * FROM (SELECT 1 AS x) t1, (SELECT 2) t2  -- derived tables
      WITH a AS (...), b AS (...) SELECT * FROM a, b   -- CTEs
      SELECT * FROM orders, customers WHERE ...        -- ANSI cross-join

  Known follow-up: ``SELECT * FROM a LEFT JOIN b ON ...`` with
  *unmatched* left rows still under-counts columns because the VM's
  ``_do_scan_all_columns`` opcode bails out when the right cursor has
  no current row instead of NULL-padding.  Matched rows are now
  correct.  Threading the cursor schema into VM state at OpenScan
  time will close that gap in a separate PR.

## [1.30.0] - 2026-05-19

### Added

- **`UpsertSpec.where_instructions`** (`ir.py`) — a pre-compiled
  instruction sequence carrying the optional SQLite conditional-upsert
  WHERE predicate.  Empty tuple means "no filter; always apply".  The
  compiler in `_compile_upsert` invokes `_compile_expr` on the resolved
  predicate when present.

## [1.29.0] - 2026-05-17

### Added

- **`Like.has_escape` field** (`ir.py`) — new `bool = False` field.  When
  `True`, the VM pops an additional stack value (the escape character)
  before pattern and value.  Three-value protocol: escape, pattern, value.

- **LIKE ESCAPE codegen** (`compiler.py`) — when the planner `Like` /
  `NotLike` node carries a non-None `escape`, the compiler emits an
  additional `LoadConst(value=escape_char)` before the `Like` instruction
  and sets `Like.has_escape = True`.

## [1.28.0] - 2026-05-17

### Fixed

- **SQLite-compatible NULL ordering in ORDER BY** — the codegen's
  `_compile_sort_key` previously defaulted every sort key to `NullsOrder.LAST`,
  regardless of direction.  SQLite (and the SQL:2003 default) treats NULL as
  smaller than every non-NULL value, so an ASC sort puts NULLs *first* and a
  DESC sort puts them *last*.  The new logic:

  | `nulls_first` | direction | resolved `NullsOrder` |
  |---------------|-----------|------------------------|
  | `True`        | any       | `FIRST`               |
  | `False`       | any       | `LAST`                |
  | `None`        | ASC       | `FIRST` ← new default |
  | `None`        | DESC      | `LAST`  ← new default |

  This matches the real `sqlite3` module byte-for-byte and removes a known
  divergence that was previously documented as "this VM places NULLs last".

## [1.27.0] - 2026-05-15

### Added

- **`SortKey.column_idx`** (`ir.py`) — New optional `int | None` field on
  `SortKey` (default `None`).  When set to a 0-based integer, the VM uses
  direct index lookup (`row[column_idx]`) rather than name-based lookup
  (`result.columns.index(column)`).  This is set for both `ORDER BY N`
  positional references and `ORDER BY alias` where the alias resolves to a
  computed expression with display name `"?"`.

### Fixed

- **`ORDER BY N` / `ORDER BY alias` sort key** (`compiler.py`) — `_to_sort_key`
  now emits `SortKey(column='', column_idx=k.positional_index)` when the
  planner marks a sort key as positional (`k.positional_index is not None`).
  Previously such sort keys fell through to the `"?"` fallback, causing the VM
  to raise `ValueError: tuple.index("?")` at runtime.

- **Hidden-column injection** (`compiler.py`, `_compile_read`) — Positional sort
  keys (`column_idx is not None`) and unnamed computed-expression sort keys
  (`col == "?"`) are correctly skipped during hidden-column injection, avoiding
  spurious extra columns.

## [1.26.0] - 2026-05-15

### Added

- **`StripTrailingColumns`** (`ir.py`) — New post-processing IR instruction with
  a single field `count: int`.  Signals the VM to remove the last `count`
  columns from the result buffer's column list and from each row's value tuple.

- **Hidden sort-key column injection** (`compiler.py`, `_compile_read`) — When an
  `ORDER BY` clause references a column not in the SELECT output, the compiler
  now:

  1. Detects the hidden sort keys by comparing `SortResult` column names against
     the `Project`'s output names (`_projection_name`).  Skips Wildcard (`SELECT
     *`) projects — those always include every table column at runtime.
  2. Appends hidden `ProjectionItem` entries to the `Project` using the original
     planner `SortKey.expr` (so the correct `LoadColumn` cursor is used, not a
     fallback to cursor 0).
  3. Prepends `SetResultSchema(extended_schema)` to the core instructions to keep
     the VM's `result.columns` in sync with the actual row width during sorting.
  4. Inserts `StripTrailingColumns(count=n_hidden)` immediately after the
     `SortResult` in the post-processing list.

  This makes `SELECT name FROM employees ORDER BY salary` work correctly: the
  result rows contain only `name`, even though `salary` was used to sort them.

## [1.25.0] - 2026-05-14

### Added

- **`InsertFromResult.returning_columns`** (`ir.py`) — New optional field
  (default `()`) on the `InsertFromResult` instruction.  When non-empty,
  the instruction signals the VM to repurpose the result buffer for RETURNING
  output after draining and inserting all source rows.  The tuple contains
  the display/source column names (one entry per RETURNING expression); plain
  `Column` references resolve to the column name, which doubles as the
  ``row_dict`` lookup key in the VM.

- **RETURNING codegen for INSERT … SELECT** (`compiler.py`,
  `_compile_insert`) — The INSERT … SELECT path now passes
  ``returning_columns`` to `InsertFromResult` when the plan node carries a
  RETURNING clause.  No ``SetResultSchema`` prefix is required because
  `InsertFromResult` sets the result schema directly.

## [1.24.0] - 2026-05-14

### Added

- **`FILTER (WHERE …)` codegen** (`compiler.py`, `_compile_aggregate`) — When an
  `AggregateItem` has a non-`None` `filter_expr`, the compiler now emits a
  conditional skip block immediately before the argument push:

      ; for each aggregate slot:
      <compile filter_expr>          ; evaluate FILTER predicate
      JumpIfFalse filter_skip_<n>    ; skip if False or NULL
      <push arg(s)>                  ; push value (and key for JSON_GROUP_OBJECT)
      UpdateAgg slot=<n>             ; accumulate
      Label filter_skip_<n>          ; target on skip path

  The operand stack is balanced on both paths — the argument is pushed only
  when the filter passes, so no extra pop is needed on the skip path.  No new
  VM instructions are required; `JumpIfFalse` already existed.

## [1.23.0] - 2026-05-14

### Added

- **`AggFunc.JSON_GROUP_ARRAY` / `AggFunc.JSON_GROUP_OBJECT`** (`ir.py`) — Two
  new aggregate enum values for JSON aggregation.  Both are automatically routed
  by the existing name-lookup in `_plan_agg_to_ir`.
- **`JSON_GROUP_OBJECT` codegen in `_compile_aggregate`** (`compiler.py`) — When
  the aggregate has a `key_arg`, the compiler now emits the key expression
  *before* the value expression in the update loop.  `UpdateAgg` for
  `JSON_GROUP_OBJECT` then pops two values (value on top, key underneath).

## [1.22.0] - 2026-05-13

### Added

- **`AggFunc.TOTAL`** (`ir.py`) — Added `TOTAL = "TOTAL"` to the `AggFunc`
  enum to represent SQLite's `TOTAL()` aggregate.  The `_plan_agg_to_ir`
  helper in the compiler automatically maps `PlanAggFunc.TOTAL` → `IrAggFunc.TOTAL`
  via name-lookup, requiring no further compiler changes.

## [1.21.0] - 2026-05-13

### Added

- **`WinFuncSpec.frame` field** (`ir.py`) — `WinFuncSpec` gains an optional
  `frame: WinFrame | None = None` field (re-exported from `sql_planner.plan`).
  When `None` the VM applies its built-in SQL-standard defaults (full partition
  when no ORDER BY, cumulative RANGE UNBOUNDED PRECEDING … CURRENT ROW when
  ORDER BY is present).  When set, the VM uses `_frame_slice` to respect the
  explicit frame bounds passed from the planner.

- **`FrameBound` and `WinFrame` re-exported from `ir.py`** — callers of
  `sql_codegen.ir` can import the two new plan types directly from the codegen
  package without reaching into `sql_planner.plan`.

- **`frame` propagated in `_to_ir_win_spec`** (`compiler.py`) — the compiler's
  window-spec conversion function now copies `spec.frame` (a `WinFrame | None`
  from the planner's `WindowFuncSpec`) verbatim into the IR `WinFuncSpec`.  No
  interpretation happens here; the VM is the sole consumer of frame semantics.

## [1.20.0] - 2026-05-13

### Added

- **`IS_DISTINCT_FROM` and `IS_NOT_DISTINCT_FROM` IR opcodes** (`ir.py`) — two new
  `BinaryOpCode` variants implement the SQL:1999 NULL-safe equality operators.
  Unlike `=` and `<>`, these operators never return NULL: they treat two NULL
  operands as equal and a NULL vs. non-NULL pair as distinct.

  Truth tables::

      NULL IS DISTINCT FROM NULL         → FALSE  (both null = same)
      NULL IS DISTINCT FROM 1            → TRUE   (one null, one not)
      1    IS DISTINCT FROM 2            → TRUE   (different values)
      1    IS DISTINCT FROM 1            → FALSE  (equal values)

      NULL IS NOT DISTINCT FROM NULL     → TRUE
      NULL IS NOT DISTINCT FROM 1        → FALSE
      1    IS NOT DISTINCT FROM 1        → TRUE
      1    IS NOT DISTINCT FROM 2        → FALSE

- **`IS_DISTINCT_FROM` / `IS_NOT_DISTINCT_FROM` entries in `_BINOP_MAP`**
  (`compiler.py`) — the planner's `AstBinaryOp.IS_DISTINCT_FROM` and
  `AstBinaryOp.IS_NOT_DISTINCT_FROM` are mapped to the corresponding VM opcodes,
  so `_compile_expr` handles them without any special-casing beyond the generic
  binary-op path.

## [1.19.0] - 2026-05-13

### Added

- **`InitAgg.distinct` field** (`ir.py`) — the `InitAgg` instruction gains a new
  `distinct: bool = False` field.  When `True` the VM initialises a `seen` set on
  the corresponding `_AggState` and deduplicates inputs before accumulation,
  implementing `COUNT(DISTINCT col)`, `SUM(DISTINCT col)`, etc.

- **`distinct` propagated to `InitAgg` in `_compile_aggregate`** (`compiler.py`) —
  the aggregate body loop now emits `InitAgg(slot=s, func=..., separator=...,
  distinct=a.distinct)` so that the VM's deduplication logic is activated when the
  planner marks an `AggregateItem` as `distinct=True`.

## [1.18.0] - 2026-05-12

### Added

- **`LoadRowId` IR instruction** (`ir.py`) — a new frozen dataclass
  `LoadRowId(cursor_id: int)` that pushes the stable integer rowid of the
  currently-positioned cursor onto the operand stack.  The VM resolves it via
  duck-typed `getattr(cursor, "rowid", None)()` so that non-supporting cursors
  (subquery cursors, file-backed backends) silently push `None` without crashing.

- **`RowIdRef` → `LoadRowId` compilation** (`compiler.py`) — `_compile_expr` now
  handles `RowIdRef(table=tbl)` by looking up the cursor ID for `tbl` in
  `ctx.alias_to_cursor` and emitting `[LoadRowId(cursor_id=cid)]`.

- **`RowIdRef` in `_column_display_name`** (`compiler.py`) — the helper that maps
  expressions to their ORDER BY sort-key names now returns `"rowid"` for
  `RowIdRef`, so that `ORDER BY rowid` and `SELECT rowid … ORDER BY rowid`
  correctly resolves the sort column from the result schema.

- **`LoadRowId` exported from `sql_codegen.__init__`** — importable directly as
  `from sql_codegen import LoadRowId`.

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

