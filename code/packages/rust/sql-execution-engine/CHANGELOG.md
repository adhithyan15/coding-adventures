# Changelog — coding-adventures-sql-execution-engine (Rust)

## [0.1.0] — 2026-03-25

### Added
- Initial implementation of the SQL execution engine.
- `DataSource` trait with `schema()` and `scan()` methods.
- `execute()` and `execute_all()` public API.
- Full SELECT pipeline: FROM → JOIN → WHERE → GROUP BY → HAVING →
  SELECT projection → DISTINCT → ORDER BY → LIMIT/OFFSET.
- Five join types: INNER, LEFT, RIGHT, FULL OUTER, CROSS.
- Expression evaluator supporting arithmetic, comparisons, BETWEEN, IN,
  LIKE (% wildcards), IS NULL, IS NOT NULL, AND, OR, NOT.
- Aggregate functions: COUNT(*), COUNT(col), SUM, AVG, MIN, MAX.
- `QueryResult` struct with column names and rows.
- `ExecutionError` enum with `TableNotFound` and `ColumnNotFound` variants.
- `SqlValue` type alias and `SqlPrimitive` enum.
- Comprehensive integration tests covering all 22 test cases.
