# Changelog

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
