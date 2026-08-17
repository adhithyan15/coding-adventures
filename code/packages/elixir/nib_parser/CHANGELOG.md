# Changelog

## 0.1.1 — 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `NibParser.create_nib_parser/0` now imports a pre-compiled grammar module (`CodingAdventures.NibParser.Grammar`) instead of `File.read`-ing `nib.grammar` from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published Hex package does not ship, so `mix deps.get` + first use would raise `File.Error` (enoent).
- Dropped the `create_nib_parser/1` optional `grammars_dir` override parameter for the same reason as `nib_lexer`'s `create_nib_lexer/1` — unused by any test or call site, and no longer meaningful once the grammar is compiled into the module. `create_nib_parser/0` now takes no arguments.

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
