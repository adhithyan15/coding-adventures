# Changelog

## 1.0.0

- **Graduate to Level 1**: replace the hand-rolled Level 0 SQL engine with the full
  five-package pipeline: `SqlTextParser` → `SqlPlanner` → `SqlOptimizer` → `SqlCodegen`
  → `SqlVm` (deps: `sql-backend`, `sql-planner`, `sql-optimizer`, `sql-codegen`,
  `sql-vm`).
- Add `MiniSqliteConnection` public API class (factory `connect()`, `Connection`,
  `Cursor`) mirroring the DB-API-2.0-inspired Level 0 contract.
- Add `SqlTextParser` — a hand-written recursive-descent SQL text parser that produces
  `SqlPlanner.Statement` objects.  Supports SELECT (with DISTINCT, aliases, JOINs,
  WHERE, GROUP BY, HAVING, ORDER BY, LIMIT/OFFSET), INSERT, UPDATE, DELETE, CREATE
  TABLE, DROP TABLE, and a broad range of scalar expressions (arithmetic, comparisons,
  logical, IS NULL/NOT NULL, BETWEEN, IN, LIKE, aggregates, scalar functions).
- Implement `normalizePlan()`: rotates `Project(Sort/Limit/Distinct(...))` trees to
  `Sort/Limit/Distinct(Project(...))` so `SqlCodegen.compilePlan()` can peel post-ops
  correctly.  Augments the Project with hidden sort-key columns when the ORDER BY key
  is not in the SELECT list; strips them from the `QueryResult` after execution.
- Implement HAVING post-filter: strips `Having` nodes between `Project` and `Aggregate`
  (required because `SqlCodegen.compileCore` only handles `Project(Aggregate)` directly)
  and re-applies the predicate against result rows via a simple `evalHavingExpr`
  evaluator.
- Implement `expandInsertColumns()`: fills in column names for bare
  `INSERT INTO table VALUES (...)` statements by querying the live backend schema.
- Implement `expandSelectStar()`: expands `SELECT *` to explicit column references from
  the backend schema before planning (SqlCodegen skips unresolved `OutputColumn.Star`).
- Fix default `NULLS FIRST` sort order for `ASC` in `SqlTextParser` to match the
  `SqlVm`'s built-in sorting semantics.
- 61 tests covering all 24 conformance fixtures (C01–C24) plus API, TXN, ERR, and
  PIPE categories.  Line coverage: >80% for Level 1 classes.
- All five pipeline JARs pre-built via leaf-to-root `BUILD` steps before `gradle test`.
- `settings.gradle.kts` includes all five sibling packages as `includeBuild` entries.
- `layout.buildDirectory = file("gradle-build")` to avoid BUILD/build collision on
  case-insensitive filesystems (see lessons.md #48).

## 0.1.0

- Add a Level 0 in-memory mini-sqlite facade.
- Support connections, cursors, qmark binding, basic DDL/DML, simple SELECT queries, and transaction snapshots.
