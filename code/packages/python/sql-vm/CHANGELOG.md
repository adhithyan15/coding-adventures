# Changelog

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
