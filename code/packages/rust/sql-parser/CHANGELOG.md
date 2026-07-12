# Changelog

All notable changes to this project will be documented in this file.

## [0.1.2] - Unreleased

### Fixed

- **A join with no `ON` condition** (a Cartesian product) now parses:
  `FROM a JOIN b` and `FROM a CROSS JOIN b`. The generated grammar required an
  `ON expr` after every join; the `ON expr` is now `Optional`. The planner already
  returns `None` for a missing `ON`, and codegen already emits every pair (no
  condition check) for a conditionless INNER join, so no downstream change was
  needed.

## [0.1.1] - Unreleased

### Fixed

- **Bare `JOIN`** (without an `INNER`/`LEFT`/… keyword) now parses. The generated
  grammar's `join_clause` required a `join_type` before `JOIN`, so `FROM a JOIN b`
  failed while `FROM a INNER JOIN b` worked. `join_type` is now `Optional`,
  matching the `sql.grammar` source (`[ join_type ]`); the planner already
  defaults a missing `join_type` to INNER, so no downstream change was needed.

## [0.1.0] - 2026-03-23

### Added

- Initial release of the SQL parser crate.
- `create_sql_parser()` factory function returning `Result<GrammarParser, String>` configured for SQL.
- `parse_sql()` convenience function returning `Result<GrammarASTNode, String>` directly.
- Loads the `sql.grammar` file at runtime from the shared `grammars/` directory.
- Parses all SQL statement types: SELECT, INSERT, UPDATE, DELETE, CREATE TABLE, DROP TABLE.
- Full expression hierarchy: OR → AND → NOT → comparison → additive → multiplicative → unary → primary.
- Comparison operators: =, !=/<>, <, >, <=, >=, BETWEEN, IN, LIKE, IS NULL, IS NOT NULL.
- GROUP BY, HAVING, ORDER BY, LIMIT/OFFSET clauses in SELECT.
- JOIN clauses: INNER, LEFT [OUTER], RIGHT [OUTER], FULL [OUTER], CROSS.
- Column constraints in CREATE TABLE: NOT NULL, PRIMARY KEY, UNIQUE, DEFAULT.
- Result-returning API for clean error propagation.
- 33 unit tests covering: all statement types, expression precedence, case-insensitive keywords, DISTINCT, NULL/TRUE/FALSE literals, function calls, qualified column references, multiple statements, trailing semicolons, BETWEEN, IN, LIKE, IS NULL, AND/OR/NOT, GROUP BY, HAVING, ORDER BY, LIMIT, arithmetic expressions, factory function, invalid SQL error path, tokenization error propagation.
