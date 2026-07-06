# Changelog - @coding-adventures/sql-planner

## [0.1.0] - 2026-07-06

### Added
- Initial TypeScript implementation of the SQL query planner.
- `plan(ast: ParsedSQL): LogicalPlan` — converts a parsed SQL AST into a
  typed `LogicalPlan` tree.
- `PlanError` class for planning-time errors (unknown table, ambiguous column,
  type mismatches).
- Full `LogicalPlan` type hierarchy:
  - `ScanNode` — full table scan
  - `FilterNode` — WHERE predicate
  - `ProjectNode` — SELECT column list with optional aliases
  - `SortNode` — ORDER BY keys with `ascending` and `nullsLast` flags
  - `LimitNode` — LIMIT/OFFSET
  - `DistinctNode` — SELECT DISTINCT
  - `AggregateNode` — GROUP BY with aggregate slots
  - `HavingNode` — HAVING predicate
  - `JoinNode` — INNER/LEFT/RIGHT/FULL JOIN with condition
  - DML: `InsertNode`, `UpdateNode`, `DeleteNode`
  - DDL: `CreateTableNode`, `DropTableNode`
  - `TransactionNode`
- Planner resolves ORDER BY `nullsLast` default: `nullsLast = ascending`
  (ASC → nulls last; DESC → nulls first).
- Hidden `__sort_<expr>` ProjectItems are injected for ORDER BY on columns
  not in the SELECT list.
- All aggregate functions (COUNT, SUM, MIN, MAX, AVG) are recognized and
  separated from scalar expression projection.
- 40+ unit tests.
