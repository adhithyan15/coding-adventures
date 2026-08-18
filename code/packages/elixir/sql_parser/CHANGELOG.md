# Changelog

## 0.1.1 — 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `create_sql_parser/0` now returns the
  pre-compiled `CodingAdventures.SqlParser.Grammar` module instead of
  `File.read`-ing `sql.grammar` from `code/grammars/` on every call. The
  old code walked out of the installed package's own directory to a
  monorepo-relative path that a published Hex package does not ship, so
  `mix deps.get` + first use would raise a `File.Error`. Dropped the
  `grammars_dir` override parameter — it is no longer meaningful now that
  the grammar is compiled in, not read from disk. This required fixing a
  gap in the shared `grammar_tools` compiler: `sql.grammar`'s
  `!("FOREIGN" "KEY")`-style negative lookahead previously crashed
  `compile-grammar` (now handled, see `grammar_tools`'s own CHANGELOG).

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
