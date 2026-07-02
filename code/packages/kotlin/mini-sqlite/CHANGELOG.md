# Changelog

## 0.2.0

- Graduate to Level 1: full SQL compilation pipeline (parse → plan → optimize → codegen → VM).
- Replace hand-rolled Level 0 interpreter with a recursive-descent SQL parser (SqlParser.kt)
  that produces sql-planner AST types, then routes through SqlPlanner → SqlOptimizer →
  SqlCodegen → SqlVm for DDL/INSERT, and through direct Kotlin evaluators for SELECT/UPDATE/DELETE.
- Direct SELECT evaluator: handles column projection, WHERE, ORDER BY, LIMIT/OFFSET, DISTINCT,
  string functions (LENGTH, UPPER, LOWER, SUBSTR, TRIM, LTRIM, RTRIM, REPLACE), math functions
  (ABS, ROUND), COALESCE, concatenation (`||`), BETWEEN, IN, NOT IN, LIKE, NOT LIKE, IS NULL,
  IS NOT NULL, unary negation, full boolean short-circuit logic.
- Direct aggregate evaluator: GROUP BY, HAVING, COUNT(*), COUNT(col), COUNT(DISTINCT col),
  SUM, AVG, MIN, MAX with correct integer/float type preservation.
- Direct UPDATE evaluator: positioned cursor-based updates via InMemoryBackend.openCursor().
- Direct DELETE evaluator: positioned cursor-based deletes via InMemoryBackend.openCursor().
- Add JaCoCo coverage enforcement: 80% instruction coverage minimum, verified in CI.
- 118 tests covering all Level 1 conformance fixtures and extensive edge cases.

## 0.1.0

- Add a Level 0 in-memory mini-sqlite facade.
- Support connections, cursors, qmark binding, basic DDL/DML, simple SELECT queries, and transaction snapshots.
