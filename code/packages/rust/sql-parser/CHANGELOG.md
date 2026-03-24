# Changelog

All notable changes to this project will be documented in this file.

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
