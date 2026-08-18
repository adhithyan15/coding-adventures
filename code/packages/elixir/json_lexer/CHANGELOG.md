# Changelog

## 0.1.1 — 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `JsonLexer.create_lexer/0` now imports a pre-compiled grammar module (`CodingAdventures.JsonLexer.Grammar`) instead of `File.read!`-ing `json.tokens` from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published Hex package does not ship, so `mix deps.get` + first use would raise `File.Error` (enoent).

## 0.1.0 — 2026-03-20

### Added
- `JsonLexer.tokenize/1` — tokenize JSON source code
- `JsonLexer.create_lexer/0` — parse json.tokens grammar
- Grammar caching via `persistent_term` for repeated use
- 16 tests covering primitives, structural tokens, compound structures, whitespace, position tracking, and errors
