# Changelog

## 0.1.1 — 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `create_lexer/0` now imports a pre-compiled grammar module (`CodingAdventures.RubyLexer.Grammar`, generated via `grammar-tools compile-tokens`) instead of `File.read!`-ing the `.tokens` file from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published Hex package does not ship, so `mix deps.get` + first use would raise `File.Error` (enoent).

## 0.1.0 — 2026-03-24

### Added
- `RubyLexer.tokenize/1` — tokenize Ruby source code from `ruby.tokens`
- `RubyLexer.create_lexer/0` — parse and return the shared Ruby token grammar
- Grammar caching via `persistent_term` for repeated calls
- Tests covering keywords, Ruby-specific operators, strings, positions, and errors
