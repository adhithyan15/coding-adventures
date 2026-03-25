# Changelog — @coding-adventures/sql-execution-engine

## [0.1.0] — 2026-03-25

### Added
- Initial implementation of the SQL execution engine.
- `DataSource` interface with `schema()` and `scan()` methods.
- `execute(sql, source)` and `executeAll(sql, source)` public API.
- Full SELECT pipeline: FROM → JOIN → WHERE → GROUP BY → HAVING →
  SELECT projection → DISTINCT → ORDER BY → LIMIT/OFFSET.
- Five join types: INNER, LEFT, RIGHT, FULL OUTER, CROSS.
- Expression evaluator supporting arithmetic, comparisons, BETWEEN, IN,
  LIKE, IS NULL, IS NOT NULL, AND, OR, NOT.
- Aggregate functions: COUNT(*), COUNT(col), SUM, AVG, MIN, MAX.
- `QueryResult` interface with column names and rows.
- `ExecutionError`, `TableNotFoundError`, `ColumnNotFoundError` classes.
- Comprehensive vitest tests covering all 22+ test cases.
