# Changelog

## [1.23.0] - 2026-05-05

### Added — DEFAULT column values (end-to-end)

Full end-to-end support for `DEFAULT <literal>` column constraints.  When a
column is declared with `DEFAULT <value>` and an INSERT omits that column,
the backend fills the row with the declared default instead of `NULL`.

**Pipeline changes:**

- **`mini-sqlite/adapter.py`** (`_col_def`) — after parsing column
  constraints, detects `DEFAULT primary` and calls `_primary()` to extract
  the literal value.  `Literal` results (integer, float, string, `None`) are
  stored directly; any non-literal expression (function call, parenthesised
  expression, etc.) is silently ignored and falls back to `NO_DEFAULT`.  The
  resulting `BackendColumnDef` now includes `default=col_default`.

- **`sql-codegen/ir.py`** — added `NO_COLUMN_DEFAULT` sentinel (`Final`
  singleton) and `default: object = NO_COLUMN_DEFAULT` field on `ColumnDef`.
  The sentinel is distinct from `sql_backend.schema.NO_DEFAULT` to keep the
  IR layer free from backend imports.

- **`sql-codegen/compiler.py`** (`_to_ir_col`) — converts the backend
  `NO_DEFAULT` sentinel to the IR `NO_COLUMN_DEFAULT`, passes all other
  values through verbatim.

- **`sql-vm/vm.py`** (`_do_create_table`) — converts `NO_COLUMN_DEFAULT` back
  to `NO_DEFAULT` when building `BackendColumnDef`, passes real default values
  through.  `InMemoryBackend._apply_defaults()` uses these values to fill
  omitted columns at INSERT time.

**Supported DEFAULT literal forms:**
  - Integer: `DEFAULT 0`, `DEFAULT 42`, `DEFAULT 1`
  - Real: `DEFAULT 3.14`
  - Text: `DEFAULT 'active'`
  - Null: `DEFAULT NULL`

**Not yet supported** (planned follow-on): `DEFAULT -1` (bare negative integer
requires grammar/adapter support for unary-minus signed literals; use
`DEFAULT (-1)` as a workaround, though this currently also falls back to
`NO_DEFAULT` since the adapter only materialises `Literal` nodes).

**Tests:** `tests/test_tier9_column_defaults.py` — 27 oracle-verified tests
across 7 test classes covering integer/null/text defaults, NOT NULL+DEFAULT,
SELECT *, UNIQUE+DEFAULT, and edge cases.  Coverage remains ≥ 91%.

## [1.22.0] - 2026-05-04

### Added — INSERT OR REPLACE, INSERT OR IGNORE, REPLACE INTO

Full end-to-end support for SQLite's conflict-resolution INSERT syntax.  Every
layer of the pipeline was extended: grammar → lexer → parser → adapter →
planner → optimizer → codegen → VM.

- **`INSERT OR REPLACE INTO t VALUES …`** — if the new row conflicts on any
  UNIQUE or PRIMARY KEY column, all conflicting existing rows are deleted and
  the new row is inserted.  Exactly matches SQLite's `INSERT OR REPLACE`
  semantics.

- **`REPLACE INTO t VALUES …`** — syntactic sugar for `INSERT OR REPLACE INTO`.
  Parsed by the new `replace_stmt` grammar rule; the adapter maps it to
  `on_conflict="REPLACE"`.

- **`INSERT OR IGNORE INTO t VALUES …`** — if the new row would violate a
  UNIQUE or PRIMARY KEY constraint, the row is silently skipped.  Rows with no
  conflict are inserted normally.

- **`INSERT OR ABORT INTO t VALUES …`** — explicit form of the default
  behaviour: raises `IntegrityError` on constraint violation.

- **`INSERT OR REPLACE / IGNORE … SELECT …`** — conflict resolution also works
  for `INSERT … SELECT` forms.

- **UNIQUE column constraints now enforced for plain `INSERT`** — a latent bug
  where `col TEXT UNIQUE` constraints were silently ignored by the in-memory
  backend (and therefore by `mini_sqlite.connect(":memory:")`) has been fixed.
  The UNIQUE flag now flows correctly through: `sql_backend.schema.ColumnDef`
  → IR `ColumnDef` (new `unique` field) → `BackendColumnDef` created by the VM
  `CreateTable` handler.

### Tests

- 17 oracle-verified tests in `tests/test_tier8_insert_conflict.py` run the
  same SQL on both mini-sqlite and the real `sqlite3` module and assert the
  results are identical.  Covers: single-key REPLACE, multi-REPLACE,
  non-key-column REPLACE, UNIQUE column REPLACE/IGNORE, mixed rows, REPLACE
  INTO shorthand, `INSERT … SELECT` forms, and ABORT (default) behaviour.

## [1.21.0] - 2026-05-04

### Added — String concatenation, JOIN USING, NATURAL JOIN

- **`||` string concatenation** — SQL's standard string-concat operator is now
  fully supported end-to-end: grammar (`sql.tokens` / `sql.grammar` via
  `CONCAT_OP = "||"`) → lexer → parser → adapter (`_additive` maps
  `CONCAT_OP → BinaryOp.CONCAT`) → planner → optimizer (constant-folds
  `'hello' || 'world' → 'helloworld'`) → codegen → VM.  NULL propagates:
  `NULL || 'x'` → NULL.

- **`JOIN … USING (col, …)`** — USING syntax is now parsed and correctly
  desugared for two-table and chained three-table join cases.  The adapter
  emits `JoinClause(using=(...))` (instead of a pre-built ON expression), and
  the planner's `_build_from_tree` resolves each USING column against the full
  accumulated scope.  This is essential for three-table chains like
  `a JOIN b USING (x) JOIN c USING (y)` where `y` may live in `a`, not `b`.
  Supports INNER, LEFT, and all other join kinds.

- **`NATURAL JOIN`** — automatically equates all shared column names between
  the left scope and the right table.  Resolved in the planner where schema
  access is available.  Falls back to CROSS JOIN when no shared columns exist
  (matching SQLite semantics).  Grammar adds `NATURAL` keyword and
  `join_type` alternative; adapter emits `JoinKind.NATURAL`.

### Fixed

- **`ConstantFolding` silent NULL for `||`** — `constant_folding.py`'s
  `_apply_binary` had no case for `BinaryOp.CONCAT`, causing Python's pattern
  matching to silently return `None` and fold `'hello' || 'world'` to
  `Literal(None)`.  Now fixed.

### Tests

- `tests/test_tier7_string_and_joins.py` — 25 new oracle-verified tests
  covering `||` (10 cases: literals, columns, NULL, WHERE, alias, constant
  folding, nullable columns), `JOIN USING` (6 cases: single-column, no
  matches, multi-column, WHERE filter, LEFT JOIN, three-table chain), and
  `NATURAL JOIN` (7 cases: single shared column, no unmatched rows, multiple
  shared columns, empty right table, no shared columns → CROSS, WHERE filter,
  aliased table), plus 2 cross-feature tests combining `||` with JOIN.

## [1.20.0] - 2026-05-04

### Added — SQLite convergence (parser + runtime)

This release closes four parser-level gaps between mini-sqlite and real SQLite,
plus two correctness fixes in the shared VM runtime.

**SELECT without FROM** (`sql.grammar`, `sql-planner`, `sql-codegen`, `adapter.py`):
- The FROM clause is now optional in the grammar (`select_stmt`).
- The planner emits `SingleRow()` when `from_` is `None`; the codegen runs
  the body exactly once with no cursor loop, no AdvanceCursor, no CloseScan.
- `SELECT 1`, `SELECT UPPER('hello')`, `SELECT 1 + 1 WHERE 1 = 1` all work.

**CAST(expr AS type)** (`sql.grammar`, `sql.tokens`, `adapter.py`):
- `CAST` is now a grammar keyword with its own `cast_expr` rule so the `AS`
  inside it is never confused with a column alias.
- Adapter maps `cast_expr` to the existing `cast` scalar function
  (`FunctionCall(name='cast', args=[expr, Literal(type_name)])`).

**Table alias without AS** (`sql.grammar`, `adapter.py`):
- `FROM employees e` now accepted in addition to `FROM employees AS e`.
- Bare-NAME alias detection uses a `saw_table_name` flag to avoid eating
  SQL keywords (WHERE, JOIN, ON …) as alias names.

**GLOB operator** (`sql.grammar`, `sql.tokens`, `adapter.py`, `sql-vm`):
- `name GLOB '*.py'` and `name NOT GLOB '*.py'` are now supported.
- Compiles to `FunctionCall(name='glob', args=[pattern, string])` in the
  `glob(pattern, string)` argument order matching SQLite's C API.
- New `glob` scalar function in `sql-vm` using `fnmatch.fnmatchcase` for
  case-sensitive Unix-style pattern matching.

**Plain JOIN (= INNER JOIN)** (`sql.grammar`, `adapter.py`):
- `join_type` is now optional in `join_clause`; a bare `JOIN` keyword
  defaults to `JoinKind.INNER`.

### Fixed

- **LIKE is now case-insensitive** (`sql-vm`) — ANSI SQL and SQLite both
  treat LIKE as case-insensitive by default for ASCII. `like_match` now
  folds both value and pattern to lowercase before the DP comparison.
- **`JumpIfFalse`/`JumpIfTrue` use SQL truthiness** (`sql-vm`) — previously
  only Python `False` was treated as falsy; now `0`, `0.0`, and `None` are
  also falsy, fixing GLOB (which returns int 0/1) in WHERE clauses.

## [1.19.0] - 2026-05-04

### Added

- **`GROUP_CONCAT` end-to-end support** (`adapter.py`) — the SQL adapter
  now recognises `GROUP_CONCAT(col)` and `GROUP_CONCAT(col, separator)`,
  emitting `AggregateExpr(func=AggFunc.GROUP_CONCAT, separator=…)`.
  - Zero or 3+ arguments raise `ProgrammingError` at parse time.
  - The separator must be a string literal; non-literal separators raise
    `ProgrammingError`.
- **15 new GROUP_CONCAT tests** (`tests/test_tier5_group_concat.py`) —
  covering default and custom separators, per-group concatenation, numeric
  column values, NULL handling (skip / all-NULL → NULL / empty table → NULL),
  oracle comparison against the real `sqlite3` module, and error cases.

## [1.18.0] - 2026-05-04

### Added

- **LAG / LEAD window functions** — `LAG(col [, offset [, default]])` and
  `LEAD(col [, offset [, default]])` are now fully supported end-to-end.
  The adapter (`adapter.py`) extracts `exprs[1:]` from the `value_list`
  grammar node into `WindowFuncExpr.extra_args`; codegen normalises these
  to an `(offset, default)` pair; the VM evaluates the offset-lookback or
  lookahead within each ordered partition.
- **NTILE(n) window function** — `NTILE(n)` divides each partition into `n`
  numbered buckets (1..n) using the standard `divmod` distribution rule.
  The integer literal `n` is parsed as the sole argument to NTILE.
- **PERCENT_RANK() window function** — `PERCENT_RANK()` computes
  `(rank − 1) / (N − 1)`.  Argument-free; only `OVER (ORDER BY ...)` is
  required.  Returns `0.0` for single-row partitions.
- **CUME_DIST() window function** — `CUME_DIST()` computes the cumulative
  distribution fraction for each row's peer group.  Also argument-free.
- **NTH_VALUE(col, n) window function** — `NTH_VALUE(col, n)` returns the
  value of `col` at the n-th row (1-indexed) of the partition.  Returns
  `NULL` when the partition has fewer than n rows.
- **Negated literal folding in window extra args** (codegen) — SQL expressions
  like `LAG(col, 1, -1)` where `-1` is parsed as `UnaryExpr(NEG, Literal(1))`
  are now constant-folded to `-1` by the codegen `_literal_val` helper,
  making negative default values work correctly.

## [1.17.0] - 2026-05-04

### Added

- **RETURNING clause** — `INSERT`, `UPDATE`, and `DELETE` statements now
  support a trailing `RETURNING col1, col2, ...` clause that returns the
  affected rows as a result set, exactly like SQLite's `RETURNING` extension.
  - **INSERT RETURNING** — returns the inserted row(s); `cursor.description`
    is set, `cursor.fetchall()` / `cursor.fetchone()` work as with SELECT.
  - **UPDATE RETURNING** — returns the post-update row values for each
    matched row.
  - **DELETE RETURNING** — captures row values *before* deletion; the rows
    are gone from the table by the time the cursor is consumed.
  - The adapter (`adapter.py`) extracts the `returning_clause` AST child and
    passes a `returning=(expr, ...)` tuple to the statement constructors.
  - 17 integration tests in `tests/test_tier4_returning.py` covering single-
    row, multi-row, single- and multi-column, description header, rowcount,
    value-persistence, and empty-result cases for all three DML statements.

## [1.16.0] - 2026-05-04

### Added

- **Correlated subquery execution** — end-to-end support for subqueries
  whose WHERE clause references columns from the enclosing query (correlated
  subqueries).  The adapter, planner, codegen, and VM cooperate to re-execute
  the inner program for each outer row with the outer cursor's current snapshot.
  Supported forms:
  - `WHERE e.col IN (SELECT ... FROM t WHERE t.x = e.col)` — correlated IN
  - `WHERE e.col NOT IN (SELECT ... FROM t WHERE t.x = e.col)` — correlated NOT IN
  - `WHERE EXISTS (SELECT 1 FROM t WHERE t.x = e.col)` — correlated EXISTS
  - `WHERE NOT EXISTS (SELECT 1 FROM t WHERE t.x = e.col)` — correlated NOT EXISTS
  - `SELECT (SELECT t.col FROM t WHERE t.x = e.col) ...` — scalar subquery
    in SELECT list (returns `NULL` when inner query yields no rows)
- **14 new integration tests** in `tests/test_tier4_correlated_subquery.py`
  covering: basic correlated IN/NOT IN/EXISTS/NOT EXISTS, no-match /
  all-match variants, scalar NULL semantics, per-row re-execution
  verification, and correlated subqueries combined with outer WHERE filters.

## [1.15.0] - 2026-05-04

### Added

- **`IN (subquery)` / `NOT IN (subquery)` execution** — the adapter
  now converts the subquery form of `IN` / `NOT IN` (previously
  `ProgrammingError("subquery in IN clause is not yet supported")`)
  to `InSubquery` / `NotInSubquery` plan-expression nodes, which flow
  through the planner, codegen, and VM.  Full SQL three-valued NULL
  logic is preserved end-to-end.
- **13 new integration tests** in `tests/test_tier3_in_subquery.py`
  covering: basic `IN` / `NOT IN`, no-match / all-match / partial-match
  sets, `NULL` test-value exclusion, `NULL` in subquery set making
  `NOT IN` return `UNKNOWN`, aggregate subqueries (`GROUP BY` / `HAVING`
  inside the inner query), combined `AND` predicates, and `HAVING`-
  level `IN` filtering.

## [1.14.0] - 2026-05-04

### Added

- **FULL [OUTER] JOIN end-to-end** — `FULL JOIN` and `FULL OUTER JOIN`
  now execute correctly through the full mini-sqlite pipeline.  All rows
  from both tables appear: matched rows carry values from both sides,
  unmatched left rows carry `NULL` for right columns, and unmatched right
  rows carry `NULL` for left columns.
- **7 new integration tests** in `test_outer_join.py`:
  `test_full_outer_join_basic`, `test_full_join_keyword_alone`,
  `test_full_outer_join_no_orphans`, `test_full_outer_join_left_empty`,
  `test_full_outer_join_right_empty`, `test_full_outer_join_where_null_right`,
  `test_full_outer_join_where_null_left`.

## [1.13.0] - 2026-05-04

### Added

- **RIGHT [OUTER] JOIN end-to-end** — `RIGHT JOIN` and `RIGHT OUTER JOIN`
  now execute correctly. Unmatched right rows appear with `NULL` for all
  left-side columns. Implemented by swapping `lft`/`rgt` in the codegen
  and reusing LEFT JOIN machinery.
- **4 new integration tests** in `test_outer_join.py`:
  `test_right_outer_join_basic`, `test_right_join_keyword_alone`,
  `test_right_outer_join_left_empty`, `test_right_outer_join_where_null_left`

## [1.12.0] - 2026-05-04

### Added

- **LEFT [OUTER] JOIN end-to-end** — `LEFT JOIN` and `LEFT OUTER JOIN`
  now execute correctly through the full mini-sqlite pipeline. Unmatched
  left rows appear with `NULL` for all right-side columns.
- **Three-way chained LEFT JOIN** — `A LEFT JOIN B LEFT JOIN C` works via
  `join_match_stack` nesting in the VM; each join level tracks its own
  match state independently.
- **GROUP BY + COUNT with LEFT JOIN** — `COUNT(right_col)` correctly
  counts zero for left rows with no right match, since `COUNT` ignores
  NULLs.
- **WHERE on join result** — predicates like `WHERE right_col IS NULL`
  (anti-join pattern) and `WHERE left_col = 'x'` apply correctly after
  LEFT JOIN.

### Fixed

- **`PredicatePushdown` outer-join safety** — the optimizer no longer
  pushes right-side WHERE predicates inside a `LEFT OUTER JOIN`. Doing
  so would filter the right scan *before* the join, destroying the
  null-padding that makes the outer join semantics correct. The fix adds
  a `JoinKind`-aware guard in `_distribute_conjuncts`:
  - `LEFT JOIN`: left-side predicates may be pushed; right-side predicates
    stay above the join.
  - `RIGHT JOIN`: right-side predicates may be pushed; left-side stay above.
  - `FULL JOIN`: no predicates pushed to either side.
  - `INNER`/`CROSS`: both sides safe to push (unchanged).

## [1.11.0] - 2026-04-29

### Added

**Numeric parameter binding (`:N` style)**

`Cursor.execute` and `Connection.execute` now accept the third PEP 249
positional paramstyle: numeric `:N` placeholders bound from a `Sequence`.
This completes the trio (`?`, `:N`, `:name`) supported by the stdlib
`sqlite3` module.

```python
conn.execute(
    "SELECT * FROM employees WHERE dept = :1 OR dept = :2",
    ("eng", "sales"),
)
```

- **`binding.substitute`** — recognises `:` followed by digits as a
  numeric placeholder.  `N` is 1-indexed: `:1` → `parameters[0]`,
  `:2` → `parameters[1]`, etc.
- **Mutual exclusion** — qmark, numeric, and named styles cannot be
  mixed in a single statement.  The error message now lists all three:
  `"cannot mix '?', ':N', and ':name' parameter styles in one statement"`.
- **Error cases** — `:0` raises `ProgrammingError("1-indexed")`;
  `:N` with `N > len(parameters)` raises `out of range`; `:N` with a
  mapping raises `numeric` (must be a sequence).
- **Repeated indices** — `:1` may appear multiple times, all binding to
  the same value.  Trailing unused values in the sequence are silently
  ignored (matching `sqlite3`).
- **`paramstyle`** docstring extended to mention all three runtime
  styles; the declared value remains `"qmark"`.

### Tests added

- `tests/test_binding.py::TestNumericParameters` — 13 unit tests:
  single/multi binding, repeated index, extra-value tolerance,
  out-of-range, zero-index, scanner safe inside literals/comments,
  multi-digit index, paramstyle exclusivity (mixing, mapping rejection),
  value type rendering.
- `tests/test_cursor.py` — 3 end-to-end tests via `Connection.execute`:
  numeric SELECT, numeric INSERT, repeated index.

## [1.10.0] - 2026-04-29

### Added

**`PRAGMA user_version` (read/write) and `PRAGMA schema_version` (read)**

Two new PRAGMAs matching real SQLite's behaviour for header-field access:

```sql
PRAGMA user_version;             -- read: returns one row (user_version,)
PRAGMA user_version = 7;         -- write: stores 7 in the header
PRAGMA schema_version;           -- read: returns one row (schema_version,)
```

- **`PRAGMA user_version`** — application-defined `u32` (0 ≤ v ≤ 2³² − 1).
  Read returns a one-row, one-column result `(user_version,)`.  Write
  validates the range and stages the change on the backend; persistent
  backends (`SqliteFileBackend`) flush via the next `commit`.  An
  out-of-range value raises `ProgrammingError`.
- **`PRAGMA schema_version`** — read-only.  Returns the schema cookie,
  which is bumped automatically on every DDL operation (`CREATE TABLE`,
  `DROP TABLE`, `CREATE INDEX`, `DROP INDEX`, …).  DML statements
  (INSERT/UPDATE/DELETE) do *not* bump it.
- **`_PRAGMA_RE` extension** — the engine's PRAGMA regex now also matches
  the assignment form `PRAGMA name = <int>` (signed integer), with the
  value captured into a new `set_value` named group.
- Backend support: relies on
  `Backend.get_user_version` / `set_user_version` /
  `get_schema_version` (added in `sql-backend` 0.11.0 and
  `storage-sqlite` 0.18.0).  Backends without these methods cause
  `PRAGMA user_version` writes to raise
  `ProgrammingError("backend does not support …")` rather than
  AttributeError.

### Tests added

- `tests/test_tier3_pragma.py::TestUserVersion` — 8 tests: default,
  description, set+read, overwrite, zero-is-valid, max u32, negative
  rejected, overflow rejected.
- `tests/test_tier3_pragma.py::TestSchemaVersion` — 6 tests: default,
  description, CREATE TABLE bumps, DROP TABLE bumps, CREATE INDEX
  bumps, DML does not bump.

## [1.9.0] - 2026-04-28

### Added

**Bytes (BLOB) parameter binding**

`bytes`, `bytearray`, and `memoryview` parameters can now be bound to `?`
placeholders.  They render as the SQLite blob-literal form `X'<hex>'`,
which round-trips through the SQL lexer (it already accepts `X'...'`
since the BLOB-type work in 1.7.0).

```python
conn.execute("INSERT INTO blobs (data) VALUES (?)", (b"\xde\xad\xbe\xef",))
```

- **`binding._to_sql_literal`** — the previous `NotSupportedError` for
  byte parameters is replaced with `f"X'{bytes(value).hex()}'"`.  The
  explicit `bytes(value)` coercion materialises a fresh object so a
  hostile `bytes` subclass overriding `.hex()` cannot inject SQL.
- **`bytearray` / `memoryview`** are coerced via `bytes(...)` and render
  identically to `bytes`.
- **Empty bytes** render as `X''` (parses as a zero-length blob).

### Tests added

- `tests/test_binding.py` — 5 new tests: bytes round-trip, empty bytes,
  bytearray, memoryview, and a hostile-subclass injection-defense test.
- `tests/test_cursor.py::test_bytes_param_round_trip` — end-to-end
  insert + select of binary data through `Connection.execute`.

### Removed

- `test_bytes_not_supported` — replaced by the round-trip tests above.

## [1.8.0] - 2026-04-28

### Added

**Named parameter binding (`:name` style)**

`Cursor.execute` and `Connection.execute` now accept a `Mapping` (e.g. `dict`)
as the *parameters* argument, in addition to the existing `Sequence` form.
When a mapping is passed, every `:identifier` placeholder in the SQL is
replaced by `parameters[identifier]` — matching the stdlib `sqlite3`
behaviour and PEP 249's `"named"` paramstyle.

```python
conn.execute(
    "SELECT name FROM employees WHERE dept = :d AND active = :active",
    {"d": "eng", "active": True},
)
```

- **`binding.substitute(sql, parameters)`** — parameter type now
  `Sequence | Mapping`.  Sequence → qmark style (`?`); mapping → named
  style (`:name`).  Mixing the two styles in one statement raises
  `ProgrammingError`.
- **Identifier rules** — `:identifier` matches `[A-Za-z_][A-Za-z0-9_]*`.
  Postgres-style casts like `a::INT` are NOT recognised as placeholders
  (the `:` is followed by another `:`, not an identifier-start
  character).  Numeric placeholders like `:1` are also NOT recognised
  (PEP 249 calls those `"numeric"` style; not yet supported).
- **NULL-safe placeholders inside literals/comments** — `:foo` inside
  `'...'`, `--...`, or `/* ... */` is left untouched, matching the
  existing `?` scanner behaviour.
- **Extra dict keys are ignored** — only keys referenced by the SQL are
  consumed; unused keys do not raise (matches `sqlite3`).
- **`Connection.execute` / `Cursor.execute`** — type signature widened
  to `Sequence[Any] | Mapping[str, Any] = ()`.
- **`engine.run`** — same signature widening; forwards the mapping
  through to `substitute`.
- **`paramstyle`** docstring clarified — the module still declares
  `"qmark"` (matching stdlib `sqlite3`) but accepts both styles at
  runtime.

### Tests added

- `tests/test_binding.py::TestNamedParameters` — 17 unit tests
  covering single/multi-named binding, repeated keys, extra-key
  tolerance, missing-key error, scanner robustness inside literals
  and comments, double-colon non-recognition, identifier rules,
  paramstyle exclusivity (mixing, wrong container types), and value
  type rendering.
- `tests/test_cursor.py` — 4 end-to-end tests via `Connection.execute`:
  named SELECT, named INSERT, missing-key error, repeated key.

## [1.7.0] - 2026-04-28

### Added — SQL Extras: Scalar Subqueries, BLOB, PRAGMA, UDFs

- **Scalar subqueries** — `(SELECT expr FROM ...)` expressions now work in
  SELECT list, WHERE, and other expression positions. Returns NULL when
  the subquery finds no rows; raises `CardinalityError` when it returns
  more than one row.
- **BLOB type** — binary data via `x'DEADBEEF'` / `X'...'` hex literal
  syntax. `SqlValue` extended to include `bytes`; `sql_type_name()` returns
  `"BLOB"` for byte values.
- **PRAGMA statements** — engine-level interception for:
  - `PRAGMA table_info(t)` — column metadata (cid, name, type, notnull,
    dflt_value, pk)
  - `PRAGMA index_list(t)` — index names and uniqueness flags
  - `PRAGMA foreign_key_list(t)` — FK constraints from the live fk_child
    registry
  - `PRAGMA table_list` — all table names in the schema
- **User-defined functions (UDFs)** — `conn.create_function(name, nargs, fn)`
  registers a Python callable; nargs=-1 for variadic. UDFs take precedence
  over built-ins.

### Fixed

- **`primary_key` now flows through to backend** — `CREATE TABLE ... PRIMARY
  KEY` column constraint was lost in the IR → VM → backend pipeline.
  `IrColumnDef` now carries `primary_key: bool`; `_do_create_table` passes it
  to `BackendColumnDef`, so `PRAGMA table_info` correctly reports pk=1.

## [1.6.0] - 2026-04-28

### Added — Phase 9: SQL Triggers (BEFORE/AFTER INSERT/UPDATE/DELETE)

- **`_create_trigger()` / `_drop_trigger()` adapter functions** — translate
  `create_trigger_stmt` / `drop_trigger_stmt` AST nodes into
  `CreateTriggerStmt` / `DropTriggerStmt` planner statements.
- **`_node_to_sql()` helper** — reconstructs body SQL from the trigger body
  AST.  Re-adds single quotes around `STRING` token values (which the lexer
  strips), normalises `new`/`old` NAME tokens to uppercase, and escapes
  embedded single quotes using SQL-standard doubling.
- **`_inject_pseudo_refs()` / `_make_trigger_executor()`** — parameter-
  substitution approach for `NEW.col` / `OLD.col` references: replaces them
  with `?` placeholders bound to the actual pre/post-update row values before
  executing the body SQL.  This avoids the cursor-lookup problem that would
  arise from creating real pseudo-tables.
- **`_split_body_sql()`** — splits trigger body SQL on the `" ; "` separator
  emitted by `_node_to_sql` for multi-statement trigger bodies.
- **`run()` new parameters** — `trigger_executor` and `trigger_depth` are
  forwarded to `sql_vm.execute()`; the executor is auto-created on top-level
  calls and re-used for nested trigger body executions.
- **`test_tier3_triggers.py`** — 44 new tests covering:
  - Grammar: parser produces `create_trigger_stmt` / `drop_trigger_stmt` nodes
    (9 tests)
  - Adapter: correct `CreateTriggerStmt` / `DropTriggerStmt` output (8 tests)
  - Backend: `InMemoryBackend` trigger storage and retrieval (8 tests)
  - Integration: end-to-end trigger correctness via `:memory:` connection
    (19 tests) including BEFORE/AFTER INSERT/UPDATE/DELETE, NEW/OLD value
    access, multi-statement bodies, trigger ordering, DROP TRIGGER, and
    transaction rollback of trigger effects.

### Fixed

- **`sql-vm`: `_do_update` old-row snapshot** — `current_row` was captured as
  a mutable reference, causing AFTER UPDATE triggers to receive the
  post-update dict in `old_row`.  Fixed by copying the dict before mutation.

## [1.5.0] - 2026-04-27

### Added — Phase 8: Window Functions (OVER / PARTITION BY)

- **`_window_func_call()` adapter function** — translates a `window_func_call`
  parse-tree node into a `WindowFuncExpr`.  Handles `COUNT(*)` (becomes
  `func="count_star"` with `arg=None`), standard `func(expr)` calls, and
  arg-free functions like `ROW_NUMBER()`.  Parses `PARTITION BY` and window
  `ORDER BY` (DESC keyword detected via token inspection).
- **`_primary()` extension** — the `window_func_call` branch is tested before
  `function_call` (matching the grammar's PEG priority rule).
- **`test_tier3_window.py`** — 41 new tests covering:
  - Grammar: parser produces `window_func_call` nodes (7 tests)
  - Adapter: `_window_func_call()` produces correct `WindowFuncExpr` (13 tests)
  - Planner: `WindowAgg` plan node structure (5 tests)
  - Integration: end-to-end SQL via `:memory:` connection (16 tests)
- **`pyproject.toml` coverage `omit`** — excludes legacy `* 2.py` duplicate
  files from coverage measurement so the 80% threshold reflects real code.

### Functions supported end-to-end

`ROW_NUMBER()`, `RANK()`, `DENSE_RANK()`, `SUM(col)`, `COUNT(*)`,
`COUNT(col)`, `AVG(col)`, `MIN(col)`, `MAX(col)`, `FIRST_VALUE(col)`,
`LAST_VALUE(col)` — all with optional `PARTITION BY` and/or `ORDER BY`
inside the `OVER (…)` clause.

## [1.4.0] - 2026-04-27

### Added — Phase 7: SAVEPOINT / RELEASE / ROLLBACK TO

- **`SAVEPOINT name`** — creates a named savepoint within the active
  transaction (implicitly begins a transaction if none is open, matching
  SQLite semantics).
- **`RELEASE [SAVEPOINT] name`** — destroys the named savepoint and all
  savepoints created after it; changes since the savepoint are kept in the
  outer transaction.
- **`ROLLBACK TO [SAVEPOINT] name`** — rolls back all changes made after
  the named savepoint.  The savepoint itself survives and can be rolled
  back to again.
- **cursor `_tcl_keyword()` fix** — `ROLLBACK TO …` is no longer
  intercepted by the TCL fast-path; it passes through to the full engine
  pipeline so the grammar can extract the savepoint name.
- **`Connection._savepoints`** — live `list[str]` tracking active
  savepoints; cleared automatically on `COMMIT` or `ROLLBACK`.
- **27 new tests** in `tests/test_tier3_savepoint.py` covering grammar,
  adapter, end-to-end integration, and error handling.

## [1.3.0] - 2026-04-27

### Added — Phase 6: CREATE / DROP VIEW

- **`CREATE VIEW [IF NOT EXISTS] name AS query`** — the engine intercepts
  `CreateViewStmt` before calling `plan()` and stores the view's defining
  `SelectStmt` in the connection's `_view_defs` dict.  `IF NOT EXISTS` silently
  skips the operation when the view already exists; without the flag an existing
  view name raises `ProgrammingError`.
- **`DROP VIEW [IF EXISTS] name`** — removes the named view from `_view_defs`.
  `IF EXISTS` is a no-op when the view is absent; without the flag a missing
  name raises `ProgrammingError("no such view: …")`.
- **View expansion in the adapter** — `to_statement()` now accepts a
  `view_defs: dict[str, SelectStmt] | None` parameter that is threaded through
  `_query_stmt` → `_select` → `_table_ref` / `_join_clause`.  A plain table
  reference whose name matches an entry in `view_defs` is expanded inline to a
  `DerivedTableRef`, exactly like a non-recursive CTE.  CTEs take priority over
  views with the same name.
- **`adapter._create_view` / `_drop_view`** helper functions parse the two new
  statement forms and produce the matching planner AST nodes.
- **23 new tests** in `tests/test_tier3_views.py` covering grammar parsing,
  adapter AST construction, view expansion, and end-to-end SQL execution.

## [1.2.0] - 2026-04-27

### Added — Phase 5b: Recursive CTEs

- **End-to-end `WITH RECURSIVE` support** — `adapter._query_stmt()` detects a
  `RECURSIVE` keyword in the `with_clause` node and, when the CTE body contains
  a `set_op_clause` (UNION / UNION ALL), parses it as a `RecursiveCTERef`
  instead of a plain `SelectStmt`.  The adapter parses the anchor sub-select
  first (with the CTE name in scope for other CTEs but not for self), then
  parses the recursive body with the CTE name excluded from `active_ctes` so
  that the self-reference resolves to a plain `TableRef` for the planner.
- **`adapter._table_ref` handles `RecursiveCTERef` entries** — when a table
  name matches a `RecursiveCTERef` key in `active_ctes`, the ref is returned
  directly (with alias applied) rather than being wrapped in a `DerivedTableRef`.
  The planner's `RecursiveCTERef` path then produces a `RecursiveCTE` plan node.
- **`adapter._select` / `_join_clause`** — `ctes` parameter type extended to
  `dict[str, SelectStmt | RecursiveCTERef] | None` so recursive CTE refs flow
  through JOIN right-hand-side table references as well.
- **22 new tests** in `tests/test_tier3_recursive_cte.py`:
  - `TestRecursiveCTEGrammar` (6 tests) — grammar and adapter: `RecursiveCTERef`
    production, anchor/recursive field contents, `union_all` flag, alias
    propagation, self-reference left as `TableRef`.
  - `TestRecursiveCTEIntegration` (11 tests) — end-to-end: simple tree traversal,
    subtree starting at a node, org-chart depth computation, UNION vs UNION ALL,
    empty anchor, leaf-only query, multiple roots, ORDER BY and LIMIT on
    recursive results, COUNT aggregate over CTE.
  - `TestRecursiveCTEErrors` (5 tests) — error handling: unknown table in
    anchor, unknown column in anchor, type mismatch in WHERE, non-existent
    recursive column, LIMIT before recursion completes.

## [1.1.0] - 2026-04-27

### Added — Phase 5a: Non-recursive CTEs

- **`adapter._query_stmt()`** extended to detect an optional `with_clause`
  child node in the parse tree.  Each `cte_def` is parsed into a `SelectStmt`
  and recorded in an `active_ctes` dict that accumulates left-to-right so
  later CTEs can reference earlier ones.
- **`adapter._table_ref(ctes=)`** — when a plain table name matches a key in
  `active_ctes`, it is rewritten to a `DerivedTableRef` (alias defaults to the
  CTE name if no explicit `AS` is given).  This means CTEs are resolved
  entirely at the adapter layer; the planner, codegen, and VM see ordinary
  derived-table (subquery) nodes and require no changes.
- **`adapter._select(ctes=)` / `_join_clause(ctes=)`** — `ctes` parameter
  threaded through so JOIN right-hand-side table refs are also resolved.
- **`test_tier3_cte.py`** — 18 new tests: 5 grammar / adapter unit tests,
  9 end-to-end integration tests, and 4 error / edge-case tests.

## [1.0.0] - 2026-04-27

### Added — Phase 4b: FOREIGN KEY constraints

- **`Connection._fk_child` / `_fk_parent: dict`** — two mutable dicts
  initialized in `__init__` and threaded through every `Cursor.execute()` →
  `engine.run()` → `vm.execute()` call so FK registrations from `CREATE TABLE`
  persist for subsequent DML.
- **`engine.run()` `fk_child` / `fk_parent` parameters** — forwarded to
  `vm.execute()`.
- **`adapter._col_def()` REFERENCES parsing** — recognises `REFERENCES table`
  and `REFERENCES table(col)` grammar variants; stores `(ref_table, ref_col)`
  tuple as `ColumnDef.foreign_key` (ref_col is `None` when not specified).
- **18 new tests** in `tests/test_tier3_foreign_keys.py`:
  - `TestForeignKeyPipeline` — grammar, adapter, codegen pipeline unit tests.
  - `TestForeignKeyIntegration` — valid inserts, NULL FK passthrough, multi-child,
    delete-after-child-removed, table-survival.
  - `TestForeignKeyErrors` — missing parent on INSERT/UPDATE, RESTRICT on DELETE,
    error message content, multi-FK column enforcement.

## [0.9.0] - 2026-04-27

### Added — Phase 4a: CHECK constraints

- **`Connection._check_registry: dict`** — mutable dict initialized to `{}` on
  connection creation and threaded through `Cursor → engine.run() → vm.execute()`.
  Mutations from `CREATE TABLE` persist in this dict across `execute()` calls.
- **`engine.run()` `check_registry` parameter** — forwarded to `vm.execute()` so
  the same dict is used for both registration (CREATE TABLE) and enforcement
  (INSERT/UPDATE).
- **`adapter._col_def()` CHECK parsing** — recognises the `CHECK ( expr )` grammar
  variant and passes the parsed expression as `check_expr` on the `ColumnDef`.
- **20 new tests** in `tests/test_tier3_check_constraints.py`:
  - `TestCheckConstraintPipeline` — unit tests for grammar, adapter, planner, codegen.
  - `TestCheckConstraintIntegration` — valid inserts, boundary values, NULL semantics,
    UPDATE enforcement, multi-column checks, compound `AND` range check.
  - `TestCheckConstraintErrors` — violation on INSERT and UPDATE, error message
    mentions the column name, compound lower/upper bound violations.

## [0.8.0] - 2026-04-27

### Added — Phase 3: ALTER TABLE ADD COLUMN

- **`ALTER TABLE t ADD [COLUMN] col_def`** — full pipeline support across all layers:
  grammar, lexer keywords, adapter, planner, codegen IR, VM execution, and the
  InMemoryBackend.  Existing rows are backfilled with NULL (or the column default
  if one is provided).

- **Grammar** (`code/grammars/sql.grammar`, `sql-lexer _grammar.py`,
  `sql-parser _grammar.py`) — added `alter_table_stmt` rule; `ALTER`, `ADD`, and
  `COLUMN` registered as SQL keywords so they tokenize as KEYWORD not NAME.

- **`sql-backend`** — added abstract `add_column(table, column)` method to
  `Backend`; `InMemoryBackend` appends the column and backfills all existing rows
  with NULL; `ColumnAlreadyExists` error class added.

- **`storage-sqlite`** — `SqliteFileBackend.add_column` raises
  `Unsupported("ALTER TABLE ADD COLUMN")` (file-format rewrite not yet
  implemented).

- **`sql-planner`** — `AlterTableStmt` AST node; `AlterTable` plan node; planner
  dispatch `_plan_alter_table`.

- **`sql-codegen`** — `AlterTable` IR instruction; compiler case
  `PlanAlterTable → AlterTable` using `_to_ir_col` for type conversion.

- **`sql-vm`** — `_do_alter_table` handler; `ColumnAlreadyExists` VM error;
  `_translate_backend_error` extended to map `be.ColumnAlreadyExists`.

- **`mini_sqlite.adapter`** — `_alter_table` parser; `alter_table_stmt` dispatch.

- **`mini_sqlite.errors.translate`** — maps `ColumnAlreadyExists` to
  `OperationalError`.

- **`test_tier3_alter_table.py`** — 16 new tests across three classes:
  - `TestAlterTablePipeline` (5 tests): grammar, adapter, planner, codegen.
  - `TestAlterTableIntegration` (9 tests): nullable add, NOT NULL, INSERT after
    ALTER, UPDATE on new column, WHERE filter, multiple columns, commit.
  - `TestAlterTableErrors` (2 tests): table-not-found, duplicate-column.

## [0.7.0] - 2026-04-27

### Added — Phase 2: EXISTS / NOT EXISTS subquery expressions

- **`EXISTS (subquery)` and `NOT EXISTS (subquery)`** — fully supported in
  `WHERE`, `HAVING`, and `SELECT` list positions.  Only uncorrelated subqueries
  are supported in this version (the subquery may not reference columns from
  the outer query).

- **Grammar** (`code/grammars/sql.grammar`) — `EXISTS "(" query_stmt ")"` added
  as an alternative in the `primary` rule, before the existing subquery-in-parens
  alternative.  `NOT EXISTS` works automatically via the existing `not_expr`
  grammar rule.

- **Adapter** (`mini_sqlite.adapter._primary`) — recognises the `EXISTS`
  keyword token and constructs an `ExistsSubquery(query=SelectStmt)` from the
  child `query_stmt` node.

- **`_flatten_project_over_aggregate`** (engine) — extended to handle
  `Project(Having(Aggregate(...)))` in addition to the pre-existing
  `Project(Aggregate(...))` case.  Without this fix, HAVING clauses with
  non-standard predicates (including EXISTS) caused an "unsupported plan node:
  Having" error during codegen.

- **`test_tier3_exists.py`** — 26 new tests across three classes:
  - `TestExistsBasic` (6 tests): grammar parsing, TRUE/FALSE result verification.
  - `TestExistsIntegration` (13 tests): WHERE, HAVING, SELECT-list, AND/OR
    combinations, filtered subqueries, LIMIT 0 subquery, empty-table cases.
  - `TestNotExistsIntegration` (7 tests): same coverage for `NOT EXISTS`.

## [0.6.1] - 2026-04-27

### Added — ML observer hook: IndexPolicy.on_query_event forwarding

- **`IndexPolicy.on_query_event(event: QueryEvent) -> None`** (optional hook) —
  documented as a third, fully optional method on the `IndexPolicy` protocol.
  When implemented by a custom policy, the advisor forwards every
  `QueryEvent` to it immediately after the drop loop completes.  This gives
  ML-based or adaptive policies access to raw runtime signals — table scanned,
  filtered columns, `rows_scanned`, `rows_returned`, `used_index`, and
  `duration_us` — so they can maintain their own feature history without
  needing to intercept the advisor's internal state.

  Detection follows the same `hasattr` / `callable` pattern already used for
  `should_drop`: a policy that does not implement `on_query_event` is simply
  never called, preserving full backward compatibility with v2-style policies.

- **`IndexAdvisor.on_query_event` restructured** — the early `return` for
  policies without `should_drop` has been replaced by a guarded `if
  callable(should_drop_fn):` block so execution always reaches the
  `on_query_event` forwarding at the end of the method, regardless of whether
  the drop loop ran.

- **`tests/test_tier3_ml_hook.py`** — 14 new tests covering:
  - Protocol surface: `HitCountPolicy` has no `on_query_event`; v2 policies
    remain backward compatible.
  - Forwarding behaviour: single and multiple events forwarded in order; the
    exact same `QueryEvent` object is passed; hook fires even when
    `should_drop` is absent; hook fires after the drop loop.
  - ML policy integration via `Connection`: policy accumulates events from
    real queries, sees `used_index` after index creation, coexists with
    `should_drop`, survives `set_policy` swaps, and exposes selectivity
    signals.

## [0.6.0] - 2026-04-23

### Added — Phase 9.7: Composite (multi-column) automatic index support (IX-8)

- **`IndexAdvisor._pair_hits: dict[tuple[str, str, str], int]`** — new
  accumulator tracking `(table, col_a, col_b)` predicate pairs observed in
  full-table scans.  Pair keys are always normalised to ascending column-name
  order to avoid double-counting `(a, b)` and `(b, a)`.

- **`IndexAdvisor._auto_index_meta: dict[str, tuple[str, tuple[str, ...]]]`** —
  maps auto-created index name → `(table, columns_tuple)`.  Replaces name
  parsing for drop-loop bookkeeping; correctly handles composite names like
  `auto_orders_user_id_status` that would confuse a `split("_", 2)` approach.

- **`IndexAdvisor._record_pair(table, col_a, col_b)` callback** — increments
  `_pair_hits` for the normalised pair key, then calls
  `_maybe_create_composite_index` when the policy threshold is reached.  Pair
  callbacks are processed **before** single-column callbacks inside `_walk` so
  that if both thresholds fire in the same observation, the composite is created
  first and the subsequent single-column check correctly skips creating a
  redundant index on the leading column.

- **`IndexAdvisor._maybe_create_composite_index(table, col_a, col_b)`** —
  creates a two-column B-tree index `auto_<table>_<col_a>_<col_b>` unless any
  existing index already has `col_a` as its leading column (which would make
  the composite redundant for leading-column-only queries).  Registers the new
  index in `_auto_index_meta`.

- **`IndexAdvisor.observe_plan` updated** — passes `pair_callback=self._record_pair`
  to `_walk`.

- **`_walk` pair callback support** — the helper now accepts an optional
  `pair_callback(table, col_a, col_b)` argument.  Inside the
  `Filter(Scan(...))` branch, all `(col_i, col_j)` pairs from the predicate
  column list are dispatched to `pair_callback` before the per-column
  `callback` calls, ensuring composite creation precedes single-column creation.
  The `IndexScan` branch now destructures `columns=idx_cols` (was `column=col`)
  and iterates the tuple.

- **`engine._extract_scan_info` updated** — the `IndexScan` match arm now
  reads `columns=cols` (was `column=col`) and returns `list(cols)`.

### Tests

- `tests/test_tier3_composite.py` — 21 new tests across three classes:
  - `TestAdvisorComposite` (8 tests) — pair hit accumulation, composite index
    creation at threshold, naming convention, skipping composite when
    single-column index on leading column already exists, no duplicate creation,
    independent columns not cross-correlated, `_auto_index_meta` population,
    pair hits reset after composite drop.
  - `TestPlannerComposite` (8 tests) — planner uses composite index for both
    columns, leading-column prefix match, non-leading column cannot use
    composite, composite preferred over single-column for two-column query,
    range on second column, lower-bound range, equality on both columns,
    BETWEEN on second column.
  - `TestCompositeIntegration` (5 tests) — full end-to-end create cycle,
    range correctness, equality correctness, `auto_index=False` has no
    composite, composite drop resets pair hits.

## [0.5.0] - 2026-04-23

### Added — Phase 9.6: Automatic index drop logic (IX-7)

- **`IndexPolicy.should_drop` optional method** — the protocol now documents
  an optional `should_drop(index_name, table, column, queries_since_last_use)`
  method.  Policies without it continue to work (the advisor detects the method
  via `hasattr`).

- **`HitCountPolicy.cold_window` parameter** — new keyword-only argument
  (default 0, which disables drop logic).  When positive, `should_drop`
  returns `True` once an auto-created index hasn't been seen in
  `queries_since_last_use >= cold_window` consecutive SELECT scans.
  Negative values raise `ValueError`.

- **`HitCountPolicy.should_drop` method** — implements the optional drop
  decision.  Always returns `False` when `cold_window == 0`; otherwise
  returns `queries_since_last_use >= cold_window`.  Accepts `index_name`,
  `table`, and `column` (unused in this implementation — custom policies
  may inspect them).

- **`IndexAdvisor.on_query_event(event: QueryEvent)` hook** — second hook on
  the advisor (alongside the existing `observe_plan`).  Called by the engine
  after each SELECT scan:
  - Increments `_query_count` (the global SELECT scan counter).
  - Records `_last_use[index_name] = _query_count` when `event.used_index`
    is a known auto-index.
  - Iterates all tracked auto-indexes and calls `policy.should_drop` on each;
    drops cold indexes via `backend.drop_index(name, if_exists=True)`.
  - Clears drop-tracking state and hit counts for dropped indexes so they
    can be re-created if the query pattern returns.
  - Drop failures are swallowed — the advisor continues running.

- **`IndexAdvisor` drop-tracking state** — three new internal fields:
  `_query_count: int`, `_last_use: dict[str, int]`,
  `_created_at: dict[str, int]`.

- **`engine.run()` wires `event_cb`** — passes `advisor.on_query_event` as
  `event_cb` to `vm.execute()` and pre-populates `filtered_columns` via
  `_extract_scan_info(optimized)`.  The callback is only set for SELECT-type
  plans; DML and DDL never advance the cold-window counter.

- **`_extract_scan_info(plan)` helper** in `engine.py` — walks the logical
  plan to extract the primary scan table and filtered column names for
  pre-populating `QueryEvent`.  Uses structural pattern matching; returns
  `("", [])` for DDL/DML.

- **`QueryEvent` re-exported** from `mini_sqlite` top-level namespace and
  added to `__all__`.

### Tests

- `tests/test_tier3_drop.py` — 42 new tests across four classes:
  - `TestHitCountPolicyColdWindow` — 10 tests for the `cold_window` parameter
    and `should_drop` semantics.
  - `TestQueryEventEmission` — 8 tests for VM-level event emission (table,
    rows_scanned, rows_returned, filtered_columns, duration_us, index usage).
  - `TestAdvisorDropLogic` — 10 tests for advisor drop loop (query counting,
    last-use tracking, drop at threshold, reset on use, non-fatal failures,
    v2-policy compatibility, hit-count reset after drop).
  - `TestDropIntegration` — 6 end-to-end tests via `mini_sqlite.connect()`
    (full create-then-drop cycle, re-creation after drop, `cold_window=0`
    never drops, `auto_index=False` has no advisor, `QueryEvent` export).

## [0.4.0] - 2026-04-22

### Added — Phase 9.5: Automatic B-tree index creation (IndexAdvisor)

- **`CREATE INDEX` / `DROP INDEX` DDL** — end-to-end support for explicit
  index management:
  - Grammar extended with `create_index_stmt` and `drop_index_stmt` rules.
  - `sql-parser` regenerated from the updated grammar.
  - `sql-planner` gained `CreateIndexStmt`, `DropIndexStmt` AST nodes and
    `CreateIndex`, `DropIndex` plan nodes.  The planner dispatches to
    `_plan_create_index` / `_plan_drop_index` which emit the new plan nodes.
  - `sql-codegen` gained `CreateIndex` and `DropIndex` IR instructions plus
    compiler lowering.
  - `sql-vm` handles `CreateIndex` and `DropIndex` by calling
    `backend.create_index` and `backend.drop_index`.
  - `adapter.py` gains `_create_index()` and `_drop_index()` helper
    functions and their dispatch cases in `_stmt_dispatch`.
  - `CREATE UNIQUE INDEX` and `CREATE INDEX IF NOT EXISTS` are both
    supported.  `DROP INDEX IF EXISTS` is supported.

- **`IndexScan` planner node** — the planner can now substitute a
  `Filter(Scan(t))` with an `IndexScan(t)` when an index covering the
  predicate column exists on the backend.  Range bounds are extracted from
  EQ / GT / GTE / LT / LTE / BETWEEN predicates.  All five optimizer passes
  (`constant_folding`, `dead_code`, `limit_pushdown`, `predicate_pushdown`,
  `projection_pruning`) handle `IndexScan` as a leaf node.

- **`IndexAdvisor`** (`mini_sqlite.advisor`) — observes every optimised
  query plan and auto-creates B-tree indexes for filtered-but-unindexed
  columns:
  - Hooks into `engine.run()` via the new `advisor` keyword parameter.
    Called with the optimised plan before code generation.
  - Walks the plan tree looking for `Filter(Scan(t), predicate)` patterns
    and records `(table, column)` hit counts.
  - Uses `auto_{table}_{column}` naming convention for created indexes.
  - Skips creation if any existing index already covers the column (first
    key match).
  - Handles `IndexAlreadyExists` from the backend gracefully (race-safe
    no-op).

- **`IndexPolicy` / `HitCountPolicy`** (`mini_sqlite.policy`) — pluggable
  decision interface for auto-index creation:
  - `IndexPolicy` — `@runtime_checkable` `Protocol` requiring `should_create(table, column, hit_count) → bool`.
  - `HitCountPolicy(threshold=3)` — creates an index when a column's
    filter-hit count reaches the configured threshold.  Default threshold 3.
    Threshold must be ≥ 1 (raises `ValueError` otherwise).
  - Any object implementing `should_create` satisfies the protocol without
    subclassing.

- **`Connection.set_policy(policy)`** — replace the active
  `IndexPolicy` on a live connection without losing accumulated hit counts.
  No-op when `auto_index=False`.

- **`connect(auto_index=True)`** — new `auto_index` keyword parameter.
  `True` (default): an `IndexAdvisor` is attached to the connection.
  `False`: no advisor; automatic index management is disabled entirely.

- **`mini_sqlite.__all__`** additions: `HitCountPolicy`, `IndexAdvisor`,
  `IndexPolicy`.

### Tests

- `tests/test_tier2_features.py` — 43 additional tests covering:
  - `TestCreateDropIndex` (8 tests): CREATE INDEX, CREATE UNIQUE INDEX,
    CREATE INDEX IF NOT EXISTS idempotence, DROP INDEX, DROP INDEX IF EXISTS,
    multi-column indexes, correctness parity (indexed vs. un-indexed).
  - `TestHitCountPolicy` (10 tests): threshold semantics, protocol
    conformance, error cases, custom policy protocol.
  - `TestIndexAdvisor` (9 tests): advisor creation, set_policy, auto-index
    naming, threshold behavior (below/at/above), no-duplicate creation,
    explicit index prevents auto creation, correctness before/after.
  - `TestConnectAutoIndex` (5 tests): `auto_index` parameter, `__all__`
    exports.

## [0.3.0] - 2026-04-21

### Added — Phase 9: Tier-2 SQL features (CASE, derived tables, chained set ops, TCL)

- **CASE expression** (`CASE WHEN … THEN … [ELSE …] END`) — both searched and
  simple CASE forms now parse and execute end-to-end.  The adapter converts
  simple CASE into equality comparisons; the codegen emits a
  `JumpIfFalse`-based chain; the VM evaluates branches lazily.  CASE can appear
  in SELECT items, WHERE predicates, ORDER BY keys, and HAVING clauses.

- **Derived tables** (`(SELECT …) AS alias` in FROM) — subqueries used as
  table sources now work end-to-end.  The adapter translates to
  `DerivedTableRef`; the planner emits a `DerivedTable` plan node with resolved
  output columns; the codegen emits `RunSubquery`; the VM executes the inner
  program against the same backend and exposes the rows via `_SubqueryCursor`.

- **Chained set operations** — `A UNION B UNION C`, `A INTERSECT B EXCEPT C`,
  etc.  The adapter builds a left-associative tree of
  `UnionStmt`/`IntersectStmt`/`ExceptStmt` nodes; the planner dispatches
  through `plan()` for each left operand so nesting resolves correctly.

- **Explicit TCL interception** — `BEGIN`, `COMMIT`, and `ROLLBACK` SQL
  statements are now intercepted in `Cursor.execute()` *before*
  `_ensure_transaction_if_needed` runs, delegating to three new
  `Connection`-level methods:
  - `_tcl_begin()` — opens a transaction; raises `OperationalError` if one is
    already active.
  - `_tcl_commit()` — commits the active transaction; raises `OperationalError`
    if none exists.
  - `_tcl_rollback()` — rolls back the active transaction; raises
    `OperationalError` if none exists.
  This prevents a double-transaction collision (the connection's implicit
  transaction opening racing with the VM's `BeginTransaction` instruction).

- **`_flatten_children()` recursion in `engine.py`** — the
  `_flatten_project_over_aggregate` helper now recurses into child plans
  (including `DerivedTable`, `Filter`, `Join`, `Union`, etc.) before processing
  the outer plan, so `Project(Aggregate(...))` patterns inside derived tables
  are correctly rewritten before codegen sees them.

### Fixed

- **INSERT with explicit column list** — `_insert()` in the adapter now
  correctly parses the column name list when an `insert_body` grammar node
  separates the column list from the values.

- **`_stmt_dispatch` routing** — statements that arrive as `query_stmt` nodes
  (the grammar's outer wrapper for SELECT + set-op tails) are now handled
  explicitly; previously only bare `select_stmt` nodes were routed, causing
  parse errors for UNION queries at the top level.

### Tests

- `tests/test_tier2_features.py` — 34 new integration tests across six classes:
  `TestCaseExpression` (11), `TestDerivedTables` (5), `TestChainedSetOps` (5),
  `TestExplicitTransactions` (4), `TestSubqueriesInWhere` (5),
  `TestCrossJoin` (4).
- Mini-sqlite total: **165 tests, 89.79% coverage**.

## [0.2.0] - 2026-04-20

### Added — Phase 8: file-backed `connect()` and byte-compatibility oracle tests

- **`mini_sqlite.connect("path.db")`** now works end-to-end against a real
  SQLite `.db` file.  Previously any non-`:memory:` path raised
  `InterfaceError`; now `connect()` routes to `SqliteFileBackend(path)` from
  the `storage_sqlite` package.  The resulting `Connection` has identical PEP
  249 semantics to the in-memory connection: `commit()`, `rollback()`,
  `execute()`, `executemany()`, context-manager auto-commit / auto-rollback,
  and `cursor()` all work.

  ```python
  with mini_sqlite.connect("app.db") as conn:
      conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)")
      conn.execute("INSERT INTO users VALUES (1, 'Alice')")
  # File is byte-compatible with sqlite3's own .db format.
  ```

- **DDL auto-commit semantics**: `Connection._ensure_transaction_if_needed`
  now begins a fresh single-statement transaction for every DDL statement
  (`CREATE TABLE`, `DROP TABLE`, `ALTER TABLE`).  `Cursor.execute` calls the
  new `Connection._post_execute()` hook after each statement; for DDL that
  hook immediately commits the single-statement transaction so schema changes
  are persisted to disk even if no DML follows.  Any previously open DML
  transaction is committed first, matching the behaviour of the stdlib
  `sqlite3` module.

- **`Connection._post_execute()`** — new internal method that auto-commits
  DDL transactions.  Non-DDL statements are a no-op.

- **`Connection._ddl_txn: bool`** — new internal flag that distinguishes a
  DDL single-statement transaction (auto-commit on `_post_execute`) from a
  normal DML transaction (user-controlled commit/rollback).

- **`tests/test_file_backend.py`** — 21 new tests in two families:

  *File-backend functional tests* (12 tests) — exercise all SQL operations
  against a real `.db` file: create/reopen database, full DDL+DML round-trip,
  SELECT with WHERE, UPDATE, DELETE, DROP TABLE, explicit commit/rollback,
  context-manager commit/rollback, NULL values, 500-row large table (exercises
  B-tree splits), multiple independent tables.

  *Byte-compatibility oracle tests* (9 tests) — use Python's stdlib `sqlite3`
  module as the reference implementation:
  - `test_oracle_mini_sqlite_writes_sqlite3_reads`: write via mini_sqlite,
    read via stdlib sqlite3 — verifies on-disk format is byte-compatible.
  - `test_oracle_sqlite3_writes_mini_sqlite_reads`: write via stdlib sqlite3,
    read via mini_sqlite — verifies mini_sqlite can parse files it did not
    produce.
  - `test_oracle_null_roundtrip`: NULL values written by mini_sqlite read as
    `None` by sqlite3.
  - `test_oracle_sqlite3_null_read_by_mini_sqlite`: NULL values written by
    sqlite3 read as `None` by mini_sqlite.
  - `test_oracle_integer_types`: full integer range (0..2⁶³−1) round-trips
    through the record layer correctly.
  - `test_oracle_text_with_special_characters`: text with quotes, Unicode,
    newlines, emojis survives the round-trip.
  - `test_oracle_schema_visible_in_sqlite3`: `sqlite_schema` written by
    mini_sqlite is visible to `sqlite3`.
  - `test_oracle_append_then_read_all`: two separate mini_sqlite sessions
    both visible to stdlib sqlite3.

- `pyproject.toml` — added `"coding-adventures-storage-sqlite"` to
  `dependencies` list.

- `BUILD` — added `-e ../storage-sqlite` to the `uv pip install` command so
  the storage-sqlite package is installed in the test environment.

### Changed

- `tests/test_module.py`: `test_connect_rejects_unknown_database` (which
  expected `InterfaceError` for a file path) replaced by
  `test_connect_file_path_creates_file` which verifies that `connect(path)`
  creates a `.db` file on disk.

## [0.1.0] - 2026-04-19

### Added

- Initial release. PEP 249 DB-API 2.0 facade over the full SQL pipeline.
- `mini_sqlite.connect(":memory:")` returns an in-memory `Connection`.
- Module globals: `apilevel="2.0"`, `threadsafety=1`, `paramstyle="qmark"`.
- `Connection` with `cursor()`, `commit()`, `rollback()`, `close()`,
  `execute()`, `executemany()`, and context manager support.
- `Cursor` with `execute()`, `executemany()`, `fetchone()`,
  `fetchmany()`, `fetchall()`, `description`, `rowcount`, iteration
  protocol, and `close()`.
- ASTNode → planner Statement adapter covering SELECT (with WHERE,
  ORDER BY, LIMIT, OFFSET, DISTINCT, GROUP BY, HAVING, aggregates,
  INNER/CROSS joins), INSERT VALUES, UPDATE, DELETE, CREATE TABLE
  [IF NOT EXISTS], DROP TABLE [IF EXISTS].
- `?` parameter binding via source-level substitution (the vendored SQL
  lexer has no QMARK token, so we escape values into SQL literals
  before handing the statement to the pipeline). Arity validated, with
  backslash-escape string literals to match the lexer's rules.
- `Project(Aggregate(...))` flattening pass in the engine so the codegen
  (which expects Aggregate as the core operator) can compile aggregate
  queries wrapped by the planner in a Project for schema uniformity.
- `INSERT INTO t VALUES (...)` without a column list resolves against
  the backend's declared schema before planning.
- PEP 249 exception hierarchy with translation from every underlying
  pipeline exception family, including lexer and parser errors →
  `ProgrammingError`.
- Output value coercion: `True`/`False` → `1`/`0` to match sqlite3.

