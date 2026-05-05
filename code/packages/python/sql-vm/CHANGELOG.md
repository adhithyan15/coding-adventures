# Changelog

## 1.15.0 — 2026-05-05

### Added

- **DEFAULT column value passthrough in `_do_create_table`** (`vm.py`) —
  `CreateTable` now reads `c.default` from each `ColumnDef` in the instruction
  and passes it to `BackendColumnDef(default=...)`.  When `c.default` is the
  IR sentinel `NO_COLUMN_DEFAULT` the VM converts it to the backend sentinel
  `NO_DEFAULT`, preserving the existing "no default declared" semantics.  Any
  other value (integer, float, string, or `None` for DEFAULT NULL) is passed
  through verbatim so `InMemoryBackend._apply_defaults()` can fill the column
  on INSERT when it is omitted by the caller.

  This closes the final gap in the DEFAULT pipeline:
  `sql-parser → adapter → sql-backend ColumnDef → IR ColumnDef → VM → InMemoryBackend`.

## 1.14.0 — 2026-05-04

### Added

- **`INSERT OR REPLACE` / `REPLACE INTO`** — when `InsertRow.on_conflict ==
  "REPLACE"`, `_do_insert()` calls the new `_replace_delete_conflicts()`
  helper before inserting.  That helper scans the target table using a
  positioned cursor (`_open_cursor` where available, `scan` as fallback) and
  deletes every existing row that shares a value on any UNIQUE or PRIMARY KEY
  column with the incoming row.  The scan-delete is single-pass because the
  backend guarantees the cursor stays live after deletion and advances to the
  next row automatically.  The same logic applies to `_do_insert_from_result`
  for `INSERT OR REPLACE … SELECT`.

- **`INSERT OR IGNORE`** — when `InsertRow.on_conflict == "IGNORE"` or
  `InsertFromResult.on_conflict == "IGNORE"`, `ConstraintViolation` from the
  backend is caught silently and the row is skipped.  Other exceptions are
  still re-raised as `IntegrityError`.

- **`_replace_delete_conflicts` helper** — pre-scans a table and deletes all
  rows conflicting with a new row on any UNIQUE/PRIMARY KEY column.  Uses
  `getattr(backend, "_open_cursor", None)` to prefer positioned cursors
  (required by `InMemoryBackend.delete()`) over read-only `scan()` iterators.
  Only non-NULL column values are checked (NULL never conflicts in SQL).

### Fixed

- **`_do_create_table` now passes `unique=c.unique` to `BackendColumnDef`**
  — the VM handler for the `CreateTable` IR instruction was building
  `BackendColumnDef` without the `unique` keyword, causing every UNIQUE column
  constraint to be silently ignored by the backend.  Non-PK UNIQUE columns
  would accept duplicate values without raising `ConstraintViolation`, making
  `INSERT OR IGNORE` unable to detect non-PK UNIQUE conflicts.

## 1.13.0 — 2026-05-04

### Added

- **`glob(pattern, string)` scalar function** (`scalar_functions.py`) —
  registers the built-in `glob` function used by the `GLOB` operator.
  Case-sensitive Unix-style pattern matching via `fnmatch.fnmatchcase`.
  Returns a Python `bool` (coerced to `1`/`0` on output) so that
  `UnaryOp.NOT` and WHERE-clause `JumpIfFalse` both work correctly with it.
  NULL arguments propagate to NULL.

### Fixed

- **`JumpIfFalse` / `JumpIfTrue` now use proper SQL truthiness** (`vm.py`) —
  previously only Python `False` (identity) was treated as falsy; now any
  value for which `not v` is true (including integer `0`, float `0.0`) is
  treated as falsy. This fixes GLOB and any other scalar predicate that
  returns an integer rather than a Python bool, and correctly handles
  `WHERE 0` / `WHERE 1` literals.

- **`like_match` is now case-insensitive for ASCII** (`operators.py`) —
  ANSI SQL and SQLite both define LIKE as case-insensitive by default for
  ASCII characters. The DP table now normalises both value and pattern to
  lowercase before comparison, preserving `%` / `_` wildcard semantics.

## 1.12.0 — 2026-05-04

### Added

- **`GROUP_CONCAT` aggregate execution** (`vm.py`) — `_do_update_agg` and
  `_do_finalize_agg` now handle `AggFunc.GROUP_CONCAT`:
  - Per-row accumulation into `_AggState.items` (a `list[str]`); NULLs are
    silently ignored; integers and whole-number floats are rendered without a
    trailing `.0` to match SQLite output.
  - Finalisation joins the list with `agg.separator`; an empty list returns
    `None` (matching SQLite's NULL-for-empty-group behaviour).
- **`items` and `separator` fields on `_AggState`** (`vm.py`) — `items`
  accumulates strings for GROUP_CONCAT; `separator` is baked in at
  `InitAgg` time and carried through to `FinalizeAgg`.
- **Implicit-single-group synthesis in `AdvanceGroupKey` handler** (`vm.py`)
  — when `has_group_by=False` and the scan produced no rows (`group_order`
  is empty), the VM synthesises the implicit `()` group so that no-GROUP-BY
  aggregates over empty tables return exactly one row of NULL/zero values,
  matching the SQL standard.
- **Lazy slot initialisation in `_do_finalize_agg`** (`vm.py`) — if the
  slot list for the current group is shorter than the requested slot index
  (because `InitAgg` was never called on an empty table), the handler
  auto-grows the list with default `_AggState` entries using the `func` and
  `separator` baked into the `FinalizeAgg` instruction.  This eliminates
  the previous `InternalError` and produces the correct zero-state result.

### Security

- **NTILE DoS prevention** (`vm.py`) — `n_buckets` is clamped to
  `max(1, min(n_raw, total_rows))` before the modulo-distribution loop,
  preventing divide-by-zero and pathological O(N²) behaviour from
  caller-supplied values ≤ 0.
- **Defense-in-depth guards** (`vm.py`) — `LAG`, `LEAD`, `NTILE`, and
  `NTH_VALUE` handlers raise `RuntimeError` on non-integer extra-arg
  values, catching any `WinFuncSpec` objects that bypass codegen validation.

## 1.11.0 — 2026-05-04

### Added

- **LAG window function** (`vm.py`) — `_do_compute_window` now handles
  `WinFunc.LAG`: returns the value of `arg_col` from the row `offset`
  positions before the current row in the sorted partition.  Returns
  `default_val` (from `extra_args[1]`) when no prior row exists at that
  distance.  Offset and default are taken from `spec.extra_args = (offset,
  default)`, normalised to `(1, None)` by the codegen if omitted.
- **LEAD window function** (`vm.py`) — mirror of LAG, looks ahead by
  `offset` positions instead of behind.
- **NTILE window function** (`vm.py`) — `WinFunc.NTILE` divides the
  partition into `n` approximately equal numbered buckets (1..n).
  Distribution matches SQLite and PostgreSQL: `q, r = divmod(len, n)`;
  the first `r` buckets get `q+1` rows, the remaining `n-r` get `q` rows.
  `n` is taken from `spec.extra_args[0]`.
- **PERCENT_RANK window function** (`vm.py`) — `WinFunc.PERCENT_RANK`
  computes `(rank − 1) / (N − 1)` where rank is the SQL RANK() value and
  N is the partition size.  Returns `0.0` when `N == 1` (avoids division
  by zero).
- **CUME_DIST window function** (`vm.py`) — `WinFunc.CUME_DIST` computes
  the cumulative distribution as `(end-of-peer-group index + 1) / N`.
  Tied rows share the same peer-group endpoint so they all receive the
  same value.
- **NTH_VALUE window function** (`vm.py`) — `WinFunc.NTH_VALUE` returns
  the value of `arg_col` at the n-th row (1-indexed) of the partition.
  Rows beyond the partition size return `NULL`.  `n` is taken from
  `spec.extra_args[0]`.

## 1.10.0 — 2026-05-04

### Added

- **`last_inserted_row` field on `_VmState`** (`vm.py`) — a
  `dict[str, SqlValue]` that is overwritten with the full row dict every time
  `_do_insert` executes an `InsertRow`.  Provides the data source for
  `LoadLastInsertedColumn`.
- **`LoadLastInsertedColumn(col)` dispatch** (`vm.py`) — `_dispatch` now
  handles `LoadLastInsertedColumn` by pushing
  `st.last_inserted_row.get(ins.col)` onto the value stack, returning `None`
  (NULL) when the column is not present.  Powers INSERT … RETURNING without
  requiring an open cursor after the insert.

## 1.9.0 — 2026-05-04

### Added

- **`outer_current_row` parameter on `execute()`** (`vm.py`) — optional
  `dict[int, dict[str, SqlValue]]` mapping outer cursor IDs to their current
  row snapshots.  Defaults to `{}` (empty).  Stored in `_VmState` for use
  by the `LoadOuterColumn` handler.
- **`_VmState.outer_current_row` field** (`vm.py`) — the outer row snapshot
  from the enclosing query; populated at construction time from `execute()`'s
  parameter.
- **`LoadOuterColumn` dispatch** (`vm.py`) — `_dispatch` routes
  `LoadOuterColumn(cursor_id, col)` to the new `_load_outer_column()` helper,
  which reads `col` from `outer_current_row[cursor_id]` and pushes the value
  (or `None` if the cursor or column is absent).
- **Correlated outer-row threading** (`vm.py`) — `_do_run_exists_subquery`,
  `_do_run_scalar_subquery`, and `_do_run_in_subquery` now call
  `execute(sub_program, backend, outer_current_row=st.current_row)` so that
  inner programs can resolve `LoadOuterColumn` against the outer scan's
  snapshot.  Each outer row gets a fresh inner execution — no caching.
- **11 new VM tests** in `tests/test_correlated_subquery.py`:
  `LoadOuterColumn` unit tests (basic, missing cursor, missing column, no
  `outer_current_row`), and end-to-end planner→codegen→VM tests for
  correlated IN, NOT IN, EXISTS, NOT EXISTS, scalar subquery, and per-row
  re-execution.

## 1.8.0 — 2026-05-04

### Added

- **`RunInSubquery` handler** (`vm.py`) — executes the embedded
  `sub_program` via a recursive `execute()` call, materializes the
  first column of all result rows into a `set`, and pushes a `bool` or
  `None` onto the value stack.  SQL three-valued NULL logic:
  - test value is `NULL` → push `None`
  - test value in non-null set → push `True` (or `False` when `negate=True`)
  - set contains `NULL` and value not found → push `None` (UNKNOWN)
  - value not found, no NULLs in set → push `False` (or `True` when `negate=True`)

## 1.7.0 — 2026-05-04

### Added

- **FULL OUTER JOIN execution** — no new VM instructions needed.  FULL JOIN
  is compiled to two passes by `sql-codegen`: Pass 1 emits left rows via
  the existing LEFT JOIN machinery; Pass 2 is a right-anti-join that emits
  only unmatched right rows.  The null-padding mechanism is identical to
  LEFT/RIGHT JOIN: a closed inner cursor returns `None` from `_load_column`.
- **4 new outer-join VM tests** in `tests/test_outer_join.py`:
  `test_full_join_all_rows_appear`, `test_full_join_left_empty`,
  `test_full_join_right_empty`, `test_full_join_no_overlap`.

## 1.6.0 — 2026-05-04

### Added

- **`join_match_stack: list[bool]`** added to `_VmState` — a stack that
  tracks, per active left row, whether any right row satisfied the JOIN
  ON condition. Supports arbitrarily nested LEFT OUTER JOINs.
- **`JoinBeginRow` handler** — appends `False` to `join_match_stack`.
- **`JoinSetMatched` handler** — sets `join_match_stack[-1] = True`.
- **`JoinIfMatched(label)` handler** — pops the stack; conditionally
  jumps to *label* if the popped value is `True`. When the stack is
  empty (defensive), pops as `False` and falls through.
- **LEFT OUTER JOIN null-padding** — no new instruction required; when
  the right scan's `CloseScan` removes the cursor from `current_row`,
  any subsequent `LoadColumn` for right-side columns returns `None`
  automatically (existing `_load_column` semantics).

## 1.5.0 — 2026-04-28

### Added

- **User-defined functions (UDFs)** — `execute()` accepts `user_functions`
  dict; `_do_call_scalar` checks user registry before built-ins. nargs=-1
  for variadic functions.
- **`RunScalarSubquery` handler** — `_do_run_scalar_subquery` executes the
  embedded sub-program, pushes the single result value, or NULL when empty.
- **`CardinalityError`** (`errors.py`) — raised when a scalar subquery
  returns more than one row; exported from `sql_vm.__init__`.
- **`primary_key` passed to `BackendColumnDef`** in `_do_create_table` —
  threads the primary-key flag through to the backend so PRAGMA table_info
  correctly reports pk=1 for primary-key columns.

## 1.4.0 — 2026-04-28

### Added — Phase 9: SQL Triggers

- **`TriggerDepthError`** (`errors.py`) — raised when trigger recursion exceeds
  depth 16; exported from `sql_vm.__init__`.
- **`_VmState.trigger_executor` / `.trigger_depth`** — optional callback and
  nesting depth injected by the façade layer; the VM calls the executor for
  each trigger that should fire without importing parsing/planning code itself.
- **`execute()` new kwargs** — `trigger_executor` and `trigger_depth` wired
  into `_VmState` construction.
- **`_fire_trigger()`** — checks depth limit, then delegates to the executor.
- **`_do_insert` / `_do_update` / `_do_delete`** — fire BEFORE and AFTER
  triggers around the actual DML call.
- **`_do_create_trigger` / `_do_drop_trigger`** — new dispatch handlers for
  `CreateTriggerDef` / `DropTriggerDef` IR instructions.

### Fixed

- **`_do_update` old-row snapshot** — `current_row[cursor_id]` was captured as
  a mutable reference; subsequent in-place `update(assignments)` mutated
  `old_row` before AFTER triggers fired, causing OLD.col to return the
  post-update value.  Fixed by calling `dict(...)` to take a shallow copy.

## 1.3.0 — 2026-04-27

### Added — Phase 8: Window Functions (OVER / PARTITION BY)

- **`_do_compute_window()` handler** — dispatched when the VM encounters a
  `ComputeWindowFunctions` instruction.  Two-pass algorithm:
  1. Converts the result buffer rows to dicts keyed by `result.columns`.
  2. Groups rows into partitions by `partition_cols` (empty key = global window).
  3. Sorts each partition by `order_cols` using a NULL-first `_win_sort_key()`.
  4. Evaluates each `WinFuncSpec` in order:
     - Ranking: `ROW_NUMBER`, `RANK`, `DENSE_RANK`
     - Aggregate: `SUM`, `COUNT` (skips NULLs), `COUNT_STAR`, `AVG`, `MIN`, `MAX`
     - Value: `FIRST_VALUE`, `LAST_VALUE`
  5. Projects rows to `output_cols` and updates `result.columns`.
- **`_win_sort_key()` / `_Descending` helpers** — NULL-first sort key; wraps
  non-NULL values in `_Descending` for DESC columns.
- **`_order_vals()` helper** — extracts ordered column values from a row dict.

## 1.2.0 — 2026-04-27

### Added — Phase 5b: Recursive CTEs

- **`_VmState.working_set_data: list[dict[str, SqlValue]]`** — stores the
  current working-set rows for the recursive iteration; populated by
  `_execute_with_cursors` before each recursive step.
- **`_execute_with_cursors(program, backend, working_set_rows)`** — private
  helper that runs a sub-program with a pre-loaded working set.  Sets
  `state.working_set_data` rather than directly populating cursor 0, so
  `OpenWorkingSetScan` can re-create a fresh cursor on each inner-loop
  entry (crucial for correctness when the self-reference appears inside a JOIN).
- **`RunRecursiveCTE` dispatch** — `_do_run_recursive_cte` implements the
  fixed-point algorithm:
  1. Execute anchor program via `execute()`; collect anchor rows as the initial
     working set.
  2. Repeat: run `recursive_program` via `_execute_with_cursors(working_rows)`;
     collect new rows; if `union_all=False` deduplicate against a `seen` set.
  3. Terminate when the working set is empty.
  4. Populate `st.cursors[cursor_id]` with a `_SubqueryCursor` over all
     accumulated rows.
- **`OpenWorkingSetScan` dispatch** — handler creates a fresh
  `_SubqueryCursor(rows=st.working_set_data)` bound to `cursor_id`.
  Each call produces an independent cursor so JOIN outer loops can exhaust
  and reopen without interfering with each other.
- **Column name normalisation** — output column names always come from the
  anchor's `result.columns`, matching the SQL standard rule that UNION output
  names are taken from the leftmost SELECT.

## 1.1.0 — 2026-04-27

### Added — Phase 4b: FOREIGN KEY constraints

- **`fk_child` / `fk_parent` parameters on `execute()`** — mutable dicts passed
  from `Connection` so FK registrations from `CREATE TABLE` persist across calls.
  `fk_child`: child_table → [(child_col, parent_table, parent_col_or_None)].
  `fk_parent`: parent_table → [(child_table, child_col, parent_col_or_None)].
- **`_VmState.fk_child` / `fk_parent`** — two new `field(default_factory=dict)`
  fields carrying both directions of the FK graph.
- **`_do_create_table` populates both registries** — for every column with a
  non-None `foreign_key` tuple, writes forward (child→parent) and reverse
  (parent→child) entries using `dict.setdefault`.
- **`_check_fk_child()`** — scans the parent table and raises `ConstraintViolation`
  when a non-NULL FK value has no matching row.  NULL passes unconditionally
  (SQL standard: NULL reference is not an error).
- **`_check_fk_parent()`** — scans the child table and raises `ConstraintViolation`
  (RESTRICT) when deleting a parent row that is still referenced.
- **`_fk_find_pk()` / `_fk_row_exists()`** — helpers: PK column discovery and
  O(n) scan predicate.
- **INSERT, UPDATE, DELETE enforcement** — `_do_insert` and `_do_update` call
  `_check_fk_child` after CHECK; `_do_delete` calls `_check_fk_parent` before
  the backend write.
- **6 new VM-level tests** in `test_dml_ddl.py`.

## 1.0.0 — 2026-04-27

### Added — Phase 4a: CHECK constraints

- **`check_registry` parameter on `execute()`** — a mutable `dict` passed in from
  `Connection` so CHECK state registered by `CREATE TABLE` persists across calls.
  The dict maps `table_name → list[(col_name, check_instrs)]`.
- **`_do_create_table` populates `check_registry`** — for each column whose IR
  `ColumnDef.check_instrs` is non-empty, an entry is written into the registry so
  subsequent INSERT/UPDATE calls can enforce it.
- **`_check_constraints()` helper** — iterates over the registry entry for the
  target table, temporarily sets `st.current_row[CHECK_CURSOR_ID] = row`, runs the
  pre-compiled instruction sequence, pops the result, and raises `ConstraintViolation`
  when the result is `False`.  NULL results pass (SQL three-valued-logic).
- **`ConstraintViolation` exports `table` and `column`** — the raised exception
  carries enough detail for the mini-sqlite layer to produce an informative error.
- **INSERT and UPDATE enforcement** — `_do_insert` validates the to-be-inserted row
  before writing; `_do_update` merges pending assignments with the current row and
  validates the merged dict before writing, preserving transactional rollback on
  violation.
- **Tests** — 4 new tests in `test_dml_ddl.py` covering valid INSERT, violating
  INSERT, violating UPDATE, and NULL passthrough.

## 0.9.0 — 2026-04-27

### Added
- `ColumnAlreadyExists` VM error — raised (and exported) when ALTER TABLE tries to
  add a column that already exists.
- `AlterTable` IR instruction dispatch — `_do_alter_table` handler calls
  `backend.add_column` and translates any `BackendError`.
- `_translate_backend_error` extended to map `be.ColumnAlreadyExists` to
  `ColumnAlreadyExists`.

## 0.8.0 — 2026-04-27

### Added — Phase 2: EXISTS / NOT EXISTS subquery expressions

- **`RunExistsSubquery` dispatch** — the VM's main dispatch loop now handles
  `RunExistsSubquery` instructions.  The handler calls `execute(ins.sub_program,
  st.backend)` in a sub-state, then pushes `True` onto the value stack if the
  result set contains at least one row, `False` otherwise.  Because `NOT
  EXISTS` is represented as `UnaryExpr(NOT, ExistsSubquery(...))`, the
  existing `NOT` unary instruction handles inversion without any extra VM
  logic.

## 0.7.0 — 2026-04-27

### Added — Date/time scalar functions + scalar MAX/MIN

- **`DATE(timevalue [, modifier...])`** — returns ISO-8601 date string
  (`YYYY-MM-DD`).  Accepts `'now'`, ISO-8601 strings, Julian Day floats,
  and Unix epoch integers as time values.

- **`TIME(timevalue [, modifier...])`** — returns time string (`HH:MM:SS`).

- **`DATETIME(timevalue [, modifier...])`** — returns combined datetime string
  (`YYYY-MM-DD HH:MM:SS`).

- **`JULIANDAY(timevalue [, modifier...])`** — returns Julian Day Number as
  float.  `JULIANDAY('2000-01-01')` → `2451544.5` (well-known constant).

- **`UNIXEPOCH(timevalue [, modifier...])`** — returns Unix epoch seconds as
  integer.  `UNIXEPOCH('1970-01-01')` → `0`.

- **`STRFTIME(format, timevalue [, modifier...])`** — formats a time value
  using C-style format specifiers.  Supports all standard `%Y`, `%m`, `%d`,
  `%H`, `%M`, `%S` plus SQLite extensions `%f` (SS.SSS), `%s` (epoch
  integer), `%J` (Julian Day), `%j` (day of year), `%W` (week number).

- **Modifiers supported** for all six functions:
  `+N days/hours/minutes/seconds/months/years`,
  `-N days/…`, `start of day/month/year`, `localtime`, `utc`.
  Leap-year clamping applied when adding months (`2024-01-31 + 1 month` →
  `2024-02-29`).

- **`MAX(a, b)`** (scalar form) — returns the greater of two arguments using
  SQLite type ordering.  NULL is treated as "less than everything":
  `MAX(1, NULL)` → `1`, `MAX(NULL, NULL)` → `NULL`.

- **`MIN(a, b)`** (scalar form) — returns the lesser of two arguments.
  `MIN(1, NULL)` → `NULL`, `MIN(NULL, NULL)` → `NULL`.

  The scalar two-argument forms are dispatched via `CallScalar` and do not
  conflict with the single-argument aggregate forms handled by
  `InitAgg`/`FinalizeAgg` opcodes.

- **`tests/test_scalar_functions.py`** — 69 new tests in `TestScalarMinMax`
  and `TestDateTimeFunctions` classes covering: format correctness, NULL
  propagation, known constants (`JULIANDAY('2000-01-01')` → `2451544.5`,
  `UNIXEPOCH('1970-01-01')` → `0`), all six modifier types, leap-year
  clamping, compound modifiers, and `STRFTIME` specifiers including `%f`,
  `%s`, `%j`.

## 0.6.0 — 2026-04-23

### Changed — Phase 9.7: Composite (multi-column) automatic index support (IX-8)

- **`_do_open_index_scan` tuple-unpack fix** — index scan bounds (`lo`, `hi`)
  are now tuples (`tuple[object, ...] | None`) instead of scalars.  The handler
  previously wrapped scalar bounds with `[ins.lo]`; it now calls `list(ins.lo)`
  directly.  This is the minimal change needed to support composite multi-column
  scans: the backend's `scan_index` receives a list of values, one per leading
  index column, rather than always a 1-element list.

## 0.5.0 — 2026-04-23

### Added

- **`QueryEvent` dataclass** — emitted by `execute()` after each SELECT scan
  via the new `event_cb` callback.  Fields:
  - `table` — the table that was scanned.
  - `filtered_columns` — column names in the WHERE predicate (pre-populated
    by the caller).
  - `rows_scanned` — total rows advanced through during the scan.
  - `rows_returned` — rows emitted to the result set via `EmitRow`.
  - `used_index` — the index name used for an index scan, or `None` for a
    full-table scan.
  - `duration_us` — wall-clock execution time in microseconds.
- **`execute()` new keyword parameters**:
  - `event_cb: Callable[[QueryEvent], None] | None` — callback invoked once
    after execution when a scan table was observed.  Replaces the global
    `set_event_listener` hook for per-execution callbacks.
  - `filtered_columns: list[str] | None` — caller-supplied column names
    forwarded into the emitted `QueryEvent`.
- **`QueryEvent` exported** from `sql_vm.__init__` and included in
  `__all__`.
- **Scan telemetry in `_VmState`** — four new fields (`scan_table`,
  `scan_index`, `rows_scanned`, `rows_returned`) accumulate metrics during
  execution.  Updated by `_do_open`, `_do_open_index_scan`, `_do_advance`,
  and the `EmitRow` handler.

## 0.4.0 — 2026-04-21

### Added

- **`RunSubquery` instruction dispatch** — new `_do_run_subquery` handler
  executes a derived-table sub-program against the same backend as the outer
  query and materialises its result rows.

- **`_SubqueryCursor` class** — an in-memory `RowIterator` backed by pre-
  materialised rows from a `RunSubquery` execution.  Stored under the derived
  table's `cursor_id` in `_VmState.cursors` so the outer scan loop's
  `AdvanceCursor` / `LoadColumn` / `CloseScan` instructions work transparently
  without any special-casing in those paths.

### Fixed

- **`row_buffer` changed from `dict[str, SqlValue]` to `list[SqlValue]`** —
  the previous dict-based buffer assigned each emitted column by name, causing
  duplicate column names (e.g. two columns both called `v` in a CROSS JOIN of
  two subqueries) to silently overwrite each other.  The new list-based buffer
  appends values positionally; `EmitRow` converts it directly to a tuple so
  column positions always match the declared result schema.  `_do_scan_all_columns`
  similarly appends values rather than keying by name.

- **`cursors` field type widened** to `dict[int, RowIterator]` (was `dict[int,
  Cursor]`) to accommodate `_SubqueryCursor` alongside normal backend cursors.

### Tests

- `tests/test_tier2_features.py` — 34 new end-to-end integration tests covering
  derived tables (`RunSubquery`), CROSS JOINs, CASE expressions (searched and
  simple), chained UNION/INTERSECT/EXCEPT, explicit transaction control, and
  subqueries in WHERE (scalar subqueries and IN subqueries).

## 0.3.0 — 2026-04-21

### Added

- **`UNION` / `INTERSECT` / `EXCEPT` execution** — full support for all six
  set-operation variants:
  - `Union ALL` — both sides are appended directly to `result_buffer`.
  - `Union DISTINCT` — `DistinctResult` deduplicates the merged buffer.
  - `CaptureLeftResult` instruction — saves `result_buffer.rows` to a new
    `left_result` field on `_VmState` and clears the buffer, allowing the right
    side to fill the buffer independently.
  - `IntersectResult(all)` — set semantics (distinct rows in both sides) when
    `all=False`; bag semantics with `min(left_count, right_count)` copies when
    `all=True`.
  - `ExceptResult(all)` — set semantics (rows in left but not right) when
    `all=False`; bag semantics with `max(0, left_count − right_count)` copies
    when `all=True`.

- **`INSERT … SELECT` execution** — `InsertFromResult` instruction drains every
  row from `result_buffer` into `backend.insert()`, clears the buffer, and
  records the count in `rows_affected`.

- **Explicit transaction support** via three new VM instructions:
  - `BeginTransaction` — calls `backend.begin_transaction()`, stores the handle
    in `_VmState.transaction_handle`.  Raises `TransactionError` if a
    transaction is already active (detected via both `_VmState.transaction_handle`
    and `backend.current_transaction()`).
  - `CommitTransaction` — resolves the handle from `_VmState.transaction_handle`
    or `backend.current_transaction()`, calls `backend.commit_transaction()`.
    Raises `TransactionError` if no active transaction exists.
  - `RollbackTransaction` — same handle-resolution strategy as commit, calls
    `backend.rollback_transaction()`.

- **`TransactionError(message)`** — new `VmError` subclass raised for nested
  `BEGIN`, `COMMIT`/`ROLLBACK` without `BEGIN`, etc.

- **`TransactionError` exported** from `sql_vm.__init__`.

### Tests

- `tests/test_tier1_features.py` — 41 new integration tests in seven classes:
  `TestUnion` (7), `TestIntersect` (7), `TestExcept` (9),
  `TestInsertSelect` (5), `TestTransactions` (5),
  `TestTransactionErrors` (4), `TestSetOpEdgeCases` (3).
- VM total: **305 tests, 83.38% coverage**.

## 0.2.0 — 2026-04-20

### Added

- **Built-in scalar functions** — new `scalar_functions` module with 40+ SQLite-compatible
  functions organised into categories:
  - *NULL-handling*: `COALESCE`, `IFNULL`, `NULLIF`, `IIF`
  - *Type inspection/casting*: `TYPEOF`, `CAST` (all SQLite affinity targets)
  - *Numeric*: `ABS`, `ROUND`, `CEIL`/`CEILING`, `FLOOR`, `SIGN`, `MOD`
  - *Math (SQLite 3.35+)*: `SQRT`, `POW`/`POWER`, `LOG`/`LN`, `LOG2`, `LOG10`, `EXP`,
    `PI`, `SIN`, `COS`, `TAN`, `ASIN`, `ACOS`, `ATAN`, `ATAN2`, `DEGREES`, `RADIANS`
  - *String*: `UPPER`, `LOWER`, `LENGTH`/`LEN`, `TRIM`, `LTRIM`, `RTRIM`,
    `SUBSTR`/`SUBSTRING`, `REPLACE`, `INSTR`, `HEX`, `UNHEX`, `QUOTE`, `CHAR`, `UNICODE`,
    `ZEROBLOB`, `SOUNDEX`
  - *Formatting*: `PRINTF`/`FORMAT` (SQLite subset: `%d`, `%f`, `%e`, `%g`, `%s`, `%q`,
    `%Q`, `%%`)
  - *Utility*: `RANDOM`, `RANDOMBLOB`, `LAST_INSERT_ROWID`

- **`CallScalar` dispatch in VM** — new `_do_call_scalar` handler in `_dispatch`
  dispatches any `CallScalar` IR instruction to the scalar function registry.  Arguments
  are popped left-to-right from the stack; the result is pushed back.

- **New error classes** (`sql_vm.errors`):
  - `UnsupportedFunction(name)` — unknown function name at runtime
  - `WrongNumberOfArguments(name, expected, got)` — arity mismatch

- **Public API additions** (`sql_vm.__init__`): `UnsupportedFunction`,
  `WrongNumberOfArguments`, `call_scalar`

- **`[tool.uv.sources]`** in `pyproject.toml` — all four local transitive dependencies
  (`sql-backend`, `sql-codegen`, `sql-planner`, `sql-optimizer`) declared as editable
  path sources so `uv run` and `uv sync` resolve correctly without PyPI.

- **200 new tests** in `tests/test_scalar_functions.py` covering every function category,
  NULL propagation, edge cases, and VM end-to-end integration via `CallScalar`.

## 0.1.0 — 2026-04-19

Initial release.

- Dispatch-loop VM `execute(program, backend)` returning a `QueryResult`
- Stack machine with separate row_buffer, cursors, and agg_table state
- Full arithmetic, logic, and comparison semantics with SQL three-valued
  NULL logic (AND/OR truth tables, NULL propagation through arithmetic
  and comparisons)
- Scan, AdvanceCursor, CloseScan — paired with label-driven loop exit
- BeginRow / EmitColumn / EmitRow for result assembly
- InitAgg / UpdateAgg / FinalizeAgg / SaveGroupKey / LoadGroupKey for
  GROUP BY and HAVING
- SortResult / LimitResult / DistinctResult post-processing
- DML: InsertRow, UpdateRows, DeleteRows
- DDL: CreateTable, DropTable
- Typed error hierarchy rooted at `VmError`

