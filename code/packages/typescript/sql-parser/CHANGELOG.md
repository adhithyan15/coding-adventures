# Changelog — sql-parser

All notable changes to this package are documented here.

## [0.1.0] — 2026-03-23

### Added

- Initial implementation of `parseSQL(source: string): ASTNode`
- `createSQLParser(source: string): ASTNode` factory function (alias for `parseSQL`)
- Grammar-driven parsing via `sql.grammar` file
- AST root rule: `program` (one or more statements separated by semicolons)
- **SELECT** statements: select list, FROM, WHERE, GROUP BY, HAVING, ORDER BY (ASC/DESC), LIMIT/OFFSET
- **JOIN** clauses: INNER, LEFT, RIGHT, FULL, CROSS JOIN with ON condition
- **INSERT INTO VALUES** with optional column list and multiple row values
- **UPDATE SET WHERE** with multiple assignments
- **DELETE FROM** with optional WHERE clause
- **CREATE TABLE** with column definitions and constraints (NOT NULL, PRIMARY KEY, UNIQUE, DEFAULT)
- **CREATE TABLE IF NOT EXISTS** guard
- **DROP TABLE** with optional IF EXISTS guard
- **Expression grammar** with full operator precedence:
  - Arithmetic: `+`, `-`, `*`, `/`, `%`
  - Comparison: `=`, `!=`, `<`, `>`, `<=`, `>=`
  - Range: `BETWEEN ... AND ...`, `NOT BETWEEN`
  - Membership: `IN (...)`, `NOT IN (...)`
  - Pattern: `LIKE`, `NOT LIKE`
  - Null check: `IS NULL`, `IS NOT NULL`
  - Boolean: `AND`, `OR`, `NOT`
  - Unary negation: `-expr`
- Case-insensitive keyword support (delegated to sql-lexer)
- Comprehensive test suite with 95%+ coverage
- `package.json`, `tsconfig.json`, `BUILD`, `README.md`
