# Changelog

## [0.19.0] - 2026-05-04

### Added

- **`SingleRow` plan node** (`plan.py`) — leaf node that yields exactly one
  empty row; used by `_plan_select` when `SelectStmt.from_` is `None`
  (SELECT without FROM).  Added to `LogicalPlan` type alias and exported
  from `__init__.py`.

### Changed

- **`SelectStmt.from_` is now optional** (`ast.py`) — the `from_` field
  changed from a required `TableRef | DerivedTableRef | RecursiveCTERef` to
  an optional one that defaults to `None`.  Field ordering was updated so
  `items` (required) precedes `from_` (optional) to satisfy Python dataclass
  rules.  Planner now emits `SingleRow()` when `from_` is absent.

## [0.18.0] - 2026-05-04

### Added

- **`GROUP_CONCAT` aggregate function** (`expr.py`, `plan.py`, `planner.py`) —
  `AggFunc.GROUP_CONCAT` added to the planner-level `AggFunc` enum, matching
  the SQL `GROUP_CONCAT(col [, separator])` function.
- **`separator` field on `AggregateExpr`** (`expr.py`) — optional
  `str | None` carrying the literal separator from `GROUP_CONCAT(col, sep)`.
  `None` means "use the SQL default `','`".
- **`separator` field on `AggregateItem`** (`plan.py`) — same semantics;
  propagated from the expression level through the plan node so the codegen
  can bake the separator into `InitAgg` at compile time.
- **`separator` propagation in the planner** (`planner.py`) — both
  `_resolve` and `_collect_aggregates` now forward the `separator` field
  when constructing `AggregateItem`.

## [0.17.0] - 2026-05-04

### Added

- **`extra_args` field on `WindowFuncExpr`** (`expr.py`) — carries zero or
  more additional literal expressions beyond the primary column argument.
  Used by multi-argument window functions: `LAG(col, offset, default)`,
  `LEAD(col, offset, default)`, `NTH_VALUE(col, n)`.
- **`extra_args` field on `WindowFuncSpec`** (`plan.py`) — same semantics
  as `WindowFuncExpr.extra_args`, carried through the plan representation
  so the codegen can extract literal constants at compile time.
- **`extra_args` propagation in the planner** (`planner.py`) — `_resolve`
  now recursively resolves all `extra_args` expressions, and
  `WindowFuncSpec` construction in `_plan_select` passes `extra_args`
  through from the source expression.

## [0.16.0] - 2026-05-04

### Added

- **`returning` field on DML AST nodes** (`ast.py`) — `InsertValuesStmt`,
  `InsertSelectStmt`, `UpdateStmt`, and `DeleteStmt` each gain a
  `returning: tuple[Expr, ...] = ()` field that carries the list of
  expressions from the SQL `RETURNING` clause.
- **`returning` field on DML plan nodes** (`plan.py`) — `Insert`, `Update`,
  and `Delete` each gain the same `returning: tuple[Expr, ...] = ()` field.
- **RETURNING resolution in the planner** (`planner.py`) — `_plan_insert`,
  `_plan_insert_select`, `_plan_update`, and `_plan_delete` all resolve
  RETURNING column references against the target table's column scope (the
  same scope used for WHERE / SET expressions), converting
  `Column(table=None, col='x')` to `Column(table='tablename', col='x')`.

## [0.15.0] - 2026-05-04

### Added

- **`CorrelatedRef(outer_alias, col)` expression** (`expr.py`) — represents a
  column reference that resolves only against the *outer* query's scope inside
  a subquery.  Distinguished from `Column` so that codegen can emit a
  `LoadOuterColumn` instruction at runtime.  Returns `False` from
  `contains_aggregate()` and is a no-op in `_collect_columns()`.
- **`outer_scope` parameter on `_plan_select()` and `_resolve()`** (`planner.py`)
  — when planning a subquery, the enclosing query's column scope is passed as
  `outer_scope`.  Column references not found in the inner scope fall back to
  `outer_scope` and become `CorrelatedRef` nodes rather than raising
  `UnknownColumn`.
- **`_resolve_column()` outer-scope fallback** (`planner.py`) — qualified
  references (`alias.col`) and bare references (`col`) both try `outer_scope`
  after failing inner-scope resolution.  Qualified references produce
  `CorrelatedRef(outer_alias=alias, col=col)`; bare references walk all
  outer-scope aliases looking for a unique owner.
- **Subquery correlated planning** (`planner.py`) — `_resolve` cases for
  `ExistsSubquery`, `ScalarSubquery`, `InSubquery`, and `NotInSubquery` now
  pass the current query's scope as `outer_scope` when recursively planning
  the inner `SelectStmt`.
- **`CorrelatedRef`** exported from `sql_planner.__init__`.
- **8 new planner unit tests** in `tests/test_planner_correlated.py`
  covering: correlated IN/EXISTS/scalar/NOT IN subqueries, bare-name outer
  scope resolution, and error cases (unknown alias, column absent from all
  scopes).

## [0.14.0] - 2026-05-04

### Added

- **`InSubquery(operand, query)` expression** (`expr.py`) — represents
  `expr IN (SELECT ...)`.  Contains the resolved inner `LogicalPlan`
  (after planner rewriting from raw `SelectStmt`).  Returns `False`
  from `contains_aggregate()` and propagates `collect_columns()` on
  `operand` only.
- **`NotInSubquery(operand, query)` expression** (`expr.py`) — same
  structure as `InSubquery` but represents `expr NOT IN (SELECT ...)`.
  Both added to the `Expr` union type.
- **Planner dispatch** (`planner.py`) — `_resolve` cases for
  `InSubquery` and `NotInSubquery`: resolve `operand` in the current
  scope, then recursively plan the inner `SelectStmt` via
  `_plan_select`, replacing the raw AST node with a `LogicalPlan`.
- **`InSubquery` and `NotInSubquery`** exported from
  `sql_planner.__init__`.

## [0.13.0] - 2026-04-28

### Added

- **`ScalarSubquery(query)` expression** (`expr.py`) — represents a
  `(SELECT ...)` in expression position. Contains the resolved inner
  `LogicalPlan`. Returns `False` from `contains_aggregate()` and is a no-op
  in `collect_columns()`.

## [0.12.0] - 2026-04-28

### Added — Phase 9: SQL Triggers

- **`CreateTriggerStmt` / `DropTriggerStmt`** (`ast.py`) — typed AST nodes for
  trigger DDL statements; added to the `Statement` union type.
- **`CreateTrigger` / `DropTrigger`** (`plan.py`) — logical plan leaf nodes;
  added to the `LogicalPlan` union; `children()` returns `()` for both.
- **Planner dispatch** (`planner.py`) — `_plan_create_trigger` and
  `_plan_drop_trigger` map the AST nodes to plan nodes; exported from
  `sql_planner.__init__`.

## [0.11.0] - 2026-04-27

### Added — Phase 8: Window Functions (OVER / PARTITION BY)

- **`WindowFuncExpr` AST expression** — frozen dataclass with `func: str`,
  `arg: Expr | None`, `partition_by: tuple[Expr, ...]`, and
  `order_by: tuple[tuple[Expr, bool], ...]` (expr, descending).  Added to
  the `Expr` union.  `contains_aggregate()` returns `False` for it;
  `_collect_columns()` walks all sub-expressions.
- **`WindowFuncSpec` plan node** — frozen dataclass capturing one window
  function: `func`, `arg_expr`, `partition_by`, `order_by`, `alias`.
- **`WindowAgg` logical plan node** — `input: LogicalPlan`,
  `specs: tuple[WindowFuncSpec, ...]`, `output_cols: tuple[str, ...]`.
  Added to the `LogicalPlan` union.
- **`_plan_select()` window path** — detects `WindowFuncExpr` items in the
  SELECT list, builds an inner `Project` materialising non-window columns
  plus dependency columns (arg / partition_by / order_by refs), then wraps
  it in `WindowAgg`.  `output_cols` = non-window output names + window
  alias names.
- **`_resolve()` extension** — handles `WindowFuncExpr`, recursively
  resolving its sub-expressions against the FROM scope.
- All new types exported via `__all__`.

## [0.10.0] - 2026-04-27

### Added — Phase 7: SAVEPOINT / RELEASE / ROLLBACK TO

- **`SavepointStmt` AST node** — frozen dataclass with `name: str`.
  Represents `SAVEPOINT name`.
- **`ReleaseSavepointStmt` AST node** — `name: str`.
  Represents `RELEASE [SAVEPOINT] name`.
- **`RollbackToStmt` AST node** — `name: str`.
  Represents `ROLLBACK TO [SAVEPOINT] name`.
- All three types added to the `Statement` union and exported via `__all__`.

## [0.9.0] - 2026-04-27

### Added — Phase 6: CREATE / DROP VIEW

- **`CreateViewStmt` AST node** (`sql_planner.ast`) — frozen dataclass
  carrying `name: str`, `query: SelectStmt`, and `if_not_exists: bool`.
  Represents `CREATE [IF NOT EXISTS] VIEW name AS query`.
- **`DropViewStmt` AST node** (`sql_planner.ast`) — frozen dataclass with
  `name: str` and `if_exists: bool`.  Represents `DROP VIEW [IF EXISTS] name`.
- Both types added to the `Statement` type union and exported from
  `sql_planner.__init__`.

## [0.8.0] - 2026-04-27

### Added — Phase 5b: Recursive CTEs

- **`RecursiveCTERef` AST node** (`sql_planner.ast`) — structured representation
  of a `WITH RECURSIVE name AS (anchor UNION [ALL] recursive)` CTE reference.
  Carries `name`, `anchor: SelectStmt`, `recursive: SelectStmt`, `union_all: bool`,
  and an optional `alias` that defaults to the CTE name.
- **`WorkingSetScan` plan node** (`sql_planner.plan`) — represents a self-reference
  inside a recursive CTE body.  Holds `alias` and `columns`.  The VM maps this to
  the current working set produced by the previous iteration.
- **`RecursiveCTE` plan node** (`sql_planner.plan`) — wraps `anchor`, `recursive`
  sub-plans, `alias`, `columns`, and `union_all` flag.  Produced when the planner
  encounters a `RecursiveCTERef` in the FROM / JOIN tree.
- **Planner dispatch for `RecursiveCTERef`** — `_plan_table_ref` detects a
  `RecursiveCTERef` entry, plans the anchor without the self-reference in scope
  and plans the recursive body with the CTE name mapped to a `WorkingSetScan`,
  then wraps both in a `RecursiveCTE` plan node.
- **All three types exported** from `sql_planner.__init__`.

## [0.7.0] - 2026-04-27

### Added
- `AlterTableStmt` AST node (`sql_planner.ast`) — structured representation of
  ALTER TABLE … ADD [COLUMN] col_def.
- `AlterTable` plan node (`sql_planner.plan`) — produced by `_plan_alter_table`.
- Planner dispatch for `AlterTableStmt` in `planner.py`.
- Both types exported from `sql_planner.__init__`.

## [0.6.0] - 2026-04-27

### Added — Phase 2: EXISTS / NOT EXISTS subquery expressions

- **`ExistsSubquery` expression node** (`sql_planner.expr`) — new `Expr`
  variant representing `EXISTS (subquery)`.  Holds `query: object` (typed as
  `object` rather than `LogicalPlan` to avoid a circular import between
  `expr.py` and `plan.py`).  Before planning the field holds a raw
  `SelectStmt`; after `_resolve()` it holds a fully-planned `LogicalPlan`.

- **`ExistsSubquery` exported from `sql_planner.__init__`** — added to both
  the import block and `__all__`.

- **`_resolve()` threaded with `schema` parameter** — signature extended to
  `_resolve(expr, scope, schema=None)`.  All internal recursive calls and all
  external call sites (`_plan_select`, `_build_from_tree`, `_plan_update`,
  `_plan_delete`) updated to forward the schema context.  Required so the
  inner SELECT inside an EXISTS can be planned against the same schema.

- **`ExistsSubquery` case in `_resolve()`** — when a pre-planner
  `ExistsSubquery(query=SelectStmt)` is encountered, `_resolve` calls
  `_plan_select(stmt, schema)` and returns a post-planner
  `ExistsSubquery(query=LogicalPlan)`.  Raises `InternalError` if called
  without a schema.

- **`contains_aggregate` / `_collect_columns` updated** — both helpers return
  `False` / no-op respectively for `ExistsSubquery` (subquery column
  references don't propagate into the outer query's column set).

## [0.5.0] - 2026-04-23

### Added — Phase 9.7: Composite (multi-column) automatic index support (IX-8)

- **`IndexScan.columns: tuple[str, ...]`** — replaces the v2 `column: str`
  single-column field.  Single-column scans produce a 1-tuple; composite scans
  produce an n-tuple matching the leading prefix of the index used.  This is a
  breaking API change; downstream packages (`sql-codegen`, `sql-optimizer`,
  `mini-sqlite`) are updated in lockstep.

- **`IndexScan.lo / IndexScan.hi` widened to `tuple[object, ...] | None`** —
  v2 stored scalar bounds; v3 stores them as tuples aligned with `columns`.
  A 1-tuple bound on a 2-column index correctly constrains only the first
  column because the backend's `scan_index` performs prefix-key comparison
  (`sort_key[:len(lo_sort)]`).

- **`_MultiColBounds` internal class** (`sql_planner.planner`) — captures the
  result of multi-column bound extraction: `matched_cols`, `lo` / `hi` tuples,
  inclusive flags, and a `residual` predicate to be re-applied after the scan.

- **`_extract_multi_column_bounds(predicate, alias, index_cols)`** helper —
  walks a predicate recursively for a given ordered list of index columns.
  For each leading column it calls the existing `_extract_index_bounds`; if
  the result is an exact-equality match it extends the chain into the residual
  for the next column; a range predicate terminates the chain.  Returns `None`
  when no column of the index is constrained.

- **`_try_index_scan` now picks the best-match index** — evaluates ALL indexes
  on a table against the predicate (previously stopped at first usable index)
  and selects the one covering the most predicate columns.  Ties are broken by
  iteration order (first declared index wins).

## [0.4.0] - 2026-04-21

### Added

- **`DerivedTableRef` AST type** (`sql_planner.ast`) — represents a subquery
  used as a table source in the FROM clause: ``(SELECT …) AS alias``.
  Contains the inner `SelectStmt` and a mandatory `alias`.

- **`DerivedTable` plan node** (`sql_planner.plan`) — a leaf in the LogicalPlan
  tree that carries the inner plan, alias, and resolved output column names.
  Produced by the planner when it encounters a `DerivedTableRef`.

- **`CaseExpr` expression node** (`sql_planner.expr`) — searched CASE expression
  with a list of (condition, result) `whens` pairs and an optional `else_` branch.
  The planner's adapter converts simple CASE (with an operand) into equality
  comparisons so the rest of the pipeline only ever sees searched CASE.

- **`_output_columns()` helper** (`sql_planner.planner`) — computes the ordered
  output column names of a finished plan tree by walking transparent wrapper
  nodes (Sort, Limit, Distinct, Having) to reach the innermost Project.  Used
  to derive the schema of a derived table at planning time.

- **`SelectStmt.from_`** and **`JoinClause.right`** type-widened to accept
  `TableRef | DerivedTableRef`, enabling subqueries in both the primary FROM
  position and in JOIN targets.

- **Chained set operations** — `UnionStmt`, `IntersectStmt`, and `ExceptStmt`
  now accept any set-op statement on their `left` field (was restricted to
  `SelectStmt`).  Left-associative chaining ``A UNION B UNION C`` is modelled
  as ``UnionStmt(UnionStmt(A, B), C)``.  The planner dispatches through the
  top-level `plan()` for the left operand, so chaining resolves correctly.

- All new types re-exported from `sql_planner.__init__`.

## [0.3.0] - 2026-04-21

### Added

- **New AST statement types** (`sql_planner.ast`):
  - `UnionStmt(left, right, all)` — `SELECT … UNION [ALL] SELECT …`
  - `IntersectStmt(left, right, all)` — `SELECT … INTERSECT [ALL] SELECT …`
  - `ExceptStmt(left, right, all)` — `SELECT … EXCEPT [ALL] SELECT …`
  - `InsertSelectStmt(table, columns, select)` — `INSERT INTO … SELECT …`
  - `BeginStmt`, `CommitStmt`, `RollbackStmt` — transaction control

- **New LogicalPlan nodes** (`sql_planner.plan`):
  - `Intersect(left, right, all)` — set intersection
  - `Except(left, right, all)` — set difference
  - `Begin`, `Commit`, `Rollback` — transaction control leaf nodes

- **Planner dispatch** updated to handle all new statement types:
  - `UnionStmt` → `Union`, `IntersectStmt` → `Intersect`, `ExceptStmt` → `Except`
  - `InsertSelectStmt` → `Insert(source=InsertSource(query=…))`
  - `BeginStmt` → `Begin`, `CommitStmt` → `Commit`, `RollbackStmt` → `Rollback`

- **`children()` helper** extended for `Intersect`, `Except`, `Begin`, `Commit`, `Rollback`.

- **`Statement` and `LogicalPlan` type unions** updated with all new variants.

- All new types re-exported from `sql_planner.__init__`.

## [0.2.0] - 2026-04-19

### Added

- `EmptyResult` leaf node (produced by `DeadCodeElimination` downstream).
- `Scan.required_columns` and `Scan.scan_limit` annotations — populated
  by optimizer passes; ignored when `None`.

## [0.1.0] - 2026-04-19

### Added

- Initial release. Structured SQL AST → LogicalPlan translator.
- Typed Statement hierarchy (`SelectStmt`, `InsertValuesStmt`, `UpdateStmt`,
  `DeleteStmt`, `CreateTableStmt`, `DropTableStmt`) as the planner's input
  interface — distinct from the parser's raw grammar ASTNode so the
  planner is not coupled to parser grammar churn.
- Full LogicalPlan node set: `Scan`, `Filter`, `Project`, `Join`,
  `Aggregate`, `Having`, `Sort`, `Limit`, `Distinct`, `Union`, `Insert`,
  `Update`, `Delete`, `CreateTable`, `DropTable`.
- Expression IR (`Expr`): `Literal`, `Column`, `BinaryExpr`, `UnaryExpr`,
  `FunctionCall`, `IsNull`, `IsNotNull`, `Between`, `In`, `NotIn`, `Like`,
  `NotLike`, `Wildcard`, `AggregateExpr`, plus `contains_aggregate` and
  `collect_columns` helpers.
- `SchemaProvider` Protocol + `InMemorySchemaProvider` dict-backed
  implementation for planner-level tests. Full backends adapt via
  `sql_backend.backend_as_schema_provider`.
- Bottom-up SELECT planner: FROM/JOIN tree → WHERE filter →
  GROUP BY / aggregate → HAVING → project → distinct → sort → limit.
- Alias-aware column resolution with ambiguity detection
  (`AmbiguousColumn`) and unknown-column / unknown-table rejection.
- INSERT / UPDATE / DELETE / CREATE TABLE / DROP TABLE planning.
- `InvalidAggregate` guard: aggregates in WHERE / DELETE WHERE /
  UPDATE WHERE are rejected.
- `plan_all` helper for multi-statement scripts.
- `children()` tree-walk helper exposed from the plan module.
- PlanError hierarchy as dataclasses for structural equality in tests.

