# Changelog

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
