# Changelog — sql-planner

All notable changes to this package will be documented in this file.

## [0.1.0] — 2026-06-30

### Added

- `SqlExpr` — recursive SQL expression enum with variants:
  `Literal`, `Column`, `BinaryOp`, `UnaryOp`, `IsNull`, `IsNotNull`,
  `Between`, `Like`, `InList`, `FunctionCall`, `Aggregate`
- `BinaryOp` — arithmetic (`+`, `-`, `*`, `/`, `%`), comparison
  (`=`, `!=`, `<`, `<=`, `>`, `>=`), logical (`AND`, `OR`), concatenation (`||`)
- `UnaryOp` — `Neg` (unary minus), `Not` (logical NOT)
- `AggFunc` — `Count`, `Sum`, `Avg`, `Min`, `Max`
- `LogicalPlan` — tree of plan nodes:
  `Scan`, `Filter`, `Project`, `Join`, `Aggregate`, `Having`,
  `Sort`, `Limit`, `Distinct`, `Union`, `Insert`, `Update`,
  `Delete`, `CreateTable`, `DropTable`
- `OutputColumn`, `JoinKind`, `SortKey`, `AggregateItem`, `Assignment`, `InsertSource`
- `PlanError` — `UnknownTable`, `UnknownColumn`, `UnsupportedStatement`,
  `ParseError`, `AmbiguousColumn`
- `plan(ast, schema) -> Result<LogicalPlan, PlanError>` — plan from pre-parsed AST
- `plan_sql(sql, schema) -> Result<LogicalPlan, PlanError>` — parse + plan in one step
- `plan_expr(node) -> Result<SqlExpr, PlanError>` — plan a standalone expression node
- Full SELECT pipeline: `Scan → Filter → Aggregate → Having → Distinct → Sort → Limit → Project`
  with `Project` always outermost (per lessons.md critical ordering requirement)
- DML planners: INSERT (with/without column list, multi-row VALUES),
  UPDATE (multiple assignments, optional WHERE), DELETE (optional WHERE)
- DDL planners: CREATE TABLE (with IF NOT EXISTS, column constraints),
  DROP TABLE (with IF EXISTS)
- JOIN support: INNER, LEFT, RIGHT, FULL, CROSS joins with ON condition
- Expression planner covering full SQL expression grammar:
  OR → AND → NOT → comparison → additive → multiplicative → unary → primary
- Schema validation: unknown tables produce `PlanError::UnknownTable`
- 58 unit tests with `MockSchema`
