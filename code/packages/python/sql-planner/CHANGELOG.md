# Changelog

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
