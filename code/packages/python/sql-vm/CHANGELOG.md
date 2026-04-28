# Changelog

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

