# Changelog

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

