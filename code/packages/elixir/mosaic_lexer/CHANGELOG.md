# Changelog

All notable changes to this package will be documented in this file.

## [0.1.1] - Unreleased

### Fixed

- Eliminated runtime grammar loading: `create_lexer/0` now imports a pre-compiled grammar module (`CodingAdventures.MosaicLexer.Grammar`) instead of `File.read!`-ing `mosaic.tokens` from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published Hex package does not ship, so `mix deps.get` + first use would raise `File.Error` (enoent). Recompiling `mosaic.tokens` via `grammar-tools compile-tokens` surfaced one pre-existing validation warning — `escapes: standard` is not yet recognized by the compiler's validator (only `"none"` is) — which is unrelated to this change (the runtime `TokenGrammar.parse/1` path never ran that validator) and does not affect lexer behavior; all 43 existing tests still pass unmodified.

## [0.1.0] - 2026-04-04

### Added

- `CodingAdventures.MosaicLexer.tokenize/1` — tokenizes Mosaic source using
  the grammar-driven lexer engine driven by `mosaic.tokens`
- `CodingAdventures.MosaicLexer.create_lexer/0` — parses `mosaic.tokens` into
  a `TokenGrammar` struct for introspection or direct reuse
- `:persistent_term` caching so the grammar is only parsed once per VM
- 28 unit tests covering: `create_lexer/0` grammar introspection; keyword
  promotion (KEYWORD tokens); NAME identifiers including hyphenated CSS-like
  names; COLOR_HEX tokens (#rgb, #rrggbb, #rrggbbaa); DIMENSION tokens (16dp,
  50%, 1.5rem); NUMBER tokens; STRING tokens; all structural delimiters (LBRACE,
  RBRACE, LANGLE, RANGLE, COLON, SEMICOLON, COMMA, DOT, EQUALS, AT); slot
  reference tokenization (@name); whitespace and comment skipping; position
  tracking; realistic snippets; error handling
