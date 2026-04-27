# Changelog

All notable changes to `coding_adventures_sql_parser` will be documented in this file.

## [0.1.0] - 2026-03-23

### Added
- Initial release
- `CodingAdventures::SqlParser.parse_sql(source)` — tokenizes SQL text with
  `sql_lexer` and parses it into a generic `ASTNode` tree using
  `GrammarDrivenParser` loaded with `sql.grammar`
- `CodingAdventures::SqlParser.create_sql_parser(source)` — lower-level entry
  point that returns the configured `GrammarDrivenParser` object before calling
  `parse`, useful for introspection or deferred parsing
- `CodingAdventures::SqlParser::SQL_GRAMMAR_PATH` constant exposing the resolved
  path to `sql.grammar`, used in tests to verify the grammar file is present
- Supports the full ANSI SQL subset defined in `sql.grammar`:
  - `SELECT` with `DISTINCT`/`ALL`, column lists, wildcard (`*`), `AS` aliases,
    `INNER`/`LEFT`/`RIGHT`/`FULL OUTER`/`CROSS JOIN`, `WHERE`, `GROUP BY`,
    `HAVING`, `ORDER BY` with `ASC`/`DESC`, and `LIMIT`/`OFFSET`
  - `INSERT INTO ... VALUES (...)` with optional column list and multi-row values
  - `UPDATE ... SET ... [WHERE ...]` with multiple comma-separated assignments
  - `DELETE FROM ... [WHERE ...]`
  - `CREATE TABLE [IF NOT EXISTS] (col_def, ...)` with type names and column
    constraints (`NOT NULL`, `NULL`, `PRIMARY KEY`, `UNIQUE`, `DEFAULT <value>`)
  - `DROP TABLE [IF EXISTS] name`
  - Multiple statements separated by semicolons; optional trailing semicolon
- Full expression support: boolean (`OR`, `AND`, `NOT`), comparisons
  (`=`, `!=`, `<>`, `<`, `>`, `<=`, `>=`), `BETWEEN ... AND ...`, `IN (...)`,
  `LIKE`, `IS NULL`, `IS NOT NULL`, arithmetic (`+`, `-`, `*`, `/`, `%`),
  unary minus, function calls (`name(args...)`), qualified column references
  (`table.column`), and parenthesised sub-expressions
- Case-insensitive keyword parsing — because `sql.tokens` uses
  `@case_insensitive true`, the `sql_lexer` normalises all keywords to
  uppercase before the parser sees them, so `select`, `SELECT`, and `Select`
  all produce identical parse trees
- Comprehensive test suite (`test/test_sql_parser.rb`) covering all statement
  types, all expression forms, case-insensitivity, and error cases; SimpleCov
  branch coverage enforced with a minimum threshold of 80%
