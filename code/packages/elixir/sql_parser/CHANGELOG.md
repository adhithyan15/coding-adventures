# Changelog

## 0.1.0 — 2026-03-23

### Added
- `SqlParser.parse_sql/1` — parse SQL source code into an AST
- `SqlParser.create_sql_parser/1` — parse sql.grammar (optional custom path)
- Grammar caching via `persistent_term` for repeated use
- Support for all ANSI SQL subset statements: SELECT, INSERT, UPDATE, DELETE,
  CREATE TABLE, DROP TABLE
- Tests covering SELECT (with WHERE, ORDER BY, GROUP BY, HAVING, LIMIT, OFFSET,
  DISTINCT, aliases, JOINs), INSERT, UPDATE, DELETE, CREATE TABLE, DROP TABLE,
  multiple statements, function calls, case-insensitive keywords, comments,
  whitespace, ASTNode helpers, and error cases
