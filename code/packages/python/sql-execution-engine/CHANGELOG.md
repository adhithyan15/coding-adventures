# Changelog — coding-adventures-sql-execution-engine

## [0.1.0] — 2026-03-25

### Added
- Initial implementation of the SQL execution engine.
- `DataSource` abstract base class with `schema()` and `scan()` methods.
- `execute(sql, source)` and `execute_all(sql, source)` public API.
- Full SELECT pipeline: FROM scan → JOIN → WHERE → GROUP BY → HAVING →
  SELECT projection → DISTINCT → ORDER BY → LIMIT/OFFSET.
- Five join types: INNER, LEFT, RIGHT, FULL OUTER, CROSS.
- Expression evaluator supporting arithmetic, comparisons, BETWEEN, IN,
  LIKE (prefix `%` patterns), IS NULL, IS NOT NULL, AND, OR, NOT, and
  aggregate function calls in HAVING.
- Aggregate functions: COUNT(*), COUNT(col), SUM, AVG, MIN, MAX.
- `QueryResult` dataclass holding column names and rows.
- Error classes: `ExecutionError`, `TableNotFoundError`, `ColumnNotFoundError`.
- Comprehensive test suite with 22+ test cases covering all features.
