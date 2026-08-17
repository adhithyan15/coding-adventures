# Changelog

## 0.1.1 — 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `create_sql_lexer/0` now returns the
  pre-compiled `CodingAdventures.SqlLexer.Grammar` module instead of
  `File.read`-ing `sql.tokens` from `code/grammars/` on every call. The old
  code walked out of the installed package's own directory to a
  monorepo-relative path that a published Hex package does not ship, so
  `mix deps.get` + first use would raise a `File.Error`. Dropped the
  `grammars_dir` override parameter — it is no longer meaningful now that
  the grammar is compiled in, not read from disk.

## 0.1.0 — 2026-03-23

### Added
- `SqlLexer.tokenize_sql/1` — tokenize SQL source code into a token list
- `SqlLexer.create_sql_lexer/1` — parse sql.tokens grammar (optional custom path)
- Grammar caching via `persistent_term` for repeated use
- Case-insensitive keyword normalization — `select` → `"SELECT"`
- Support for `-- line comments` and `/* block comments */` (silently skipped)
- 50+ tests covering keywords, case normalization, identifiers, numbers, strings,
  operators, punctuation, comment skipping, compound expressions, whitespace,
  position tracking, EOF, and error cases
