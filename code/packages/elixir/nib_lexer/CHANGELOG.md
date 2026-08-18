# Changelog

## 0.1.1 — 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `NibLexer.create_nib_lexer/0` now imports a pre-compiled grammar module (`CodingAdventures.NibLexer.Grammar`) instead of `File.read`-ing `nib.tokens` from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published Hex package does not ship, so `mix deps.get` + first use would raise `File.Error` (enoent).
- Dropped the `create_nib_lexer/1` optional `grammars_dir` override parameter — it existed only to point at custom fixture directories, but no test or call site in this package (or its `nib_parser` sibling) actually used it with a non-default value, and there is no longer a grammars directory to override once the grammar is compiled into the module. `create_nib_lexer/0` now takes no arguments.

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
