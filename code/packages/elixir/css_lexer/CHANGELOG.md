# Changelog

## 0.1.1 — 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `CssLexer.create_lexer/0` now imports a pre-compiled grammar module (`CodingAdventures.CssLexer.Grammar`) instead of `File.read!`-ing `css.tokens` from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published Hex package does not ship, so `mix deps.get` + first use would raise `File.Error` (enoent).

## 0.1.0 — 2026-03-24

### Added
- `CssLexer.tokenize/1` — tokenize CSS source code from `css.tokens`
- `CssLexer.create_lexer/0` — parse and return the shared CSS token grammar
- Grammar caching via `persistent_term` for repeated calls
- Tests covering CSS compound tokens, functions, selectors, operators, positions, and errors
