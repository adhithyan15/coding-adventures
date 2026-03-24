# Changelog — sql-parser (Go)

## [0.1.0] — 2026-03-23

### Added
- `CreateSQLParser(source string)` — tokenizes SQL with the sql-lexer, loads `sql.grammar`, and returns a configured `GrammarParser`
- `ParseSQL(source string)` — convenience one-shot parse function; returns `*parser.ASTNode` rooted at "program"
- Case-insensitive keyword support via sql-lexer (SELECT == select == Select)
- Full ANSI SQL subset: SELECT (with WHERE, GROUP BY, HAVING, ORDER BY, LIMIT/OFFSET, all JOIN types), INSERT, UPDATE, DELETE, CREATE TABLE, DROP TABLE
- Expression grammar with correct precedence: OR → AND → NOT → comparison → additive → multiplicative → unary → primary
- Special SQL expression forms: BETWEEN…AND, IN(…), LIKE, IS NULL, IS NOT NULL, NOT BETWEEN, NOT IN, NOT LIKE
- Function call syntax: `name(args)` and `name(*)`
- Column/table qualified references: `schema.table`, `table.column`
- 20 unit tests covering all statement types, expressions, invalid SQL detection, case-insensitive parsing, and AST structure verification
