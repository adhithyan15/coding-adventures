# Changelog — coding-adventures-xml-lexer (Lua)

All notable changes to this package are documented here.

## Unreleased

### Fixed

- Eliminated runtime grammar loading: `tokenize` now requires a
  pre-compiled `_grammar` module instead of reading and parsing the
  `xml.tokens` file from `code/grammars/` on every call. The old code
  walked out of the installed package's own directory to a
  monorepo-relative path that a published LuaRocks package does not ship.

## [0.1.1] — 2026-08-07

### Changed
- **Now loads the canonical `code/grammars/xml/xml.tokens`** instead of a
  Lua-specific `xml_lua.tokens` fork. The fork existed only to avoid
  lookaround (unsupported by Lua patterns) and top-level `A|B` alternation
  (Lua patterns have no alternation operator at all); the canonical file
  now uses the same lookaround-free, alternation-free technique the fork
  used, so one source now serves every consuming language. See
  `coding-adventures-xml-parser`'s (Rust) CHANGELOG for the full
  rationale. No behavior change for this package — `xml_lua.tokens` and
  the new `xml.tokens` are equivalent in every way that mattered here.

### Fixed
- **Fixed a latent PI-body mis-tokenization bug** that predates this
  release: `<?t a?b?>` had the `b` after the bare `?` wrongly
  re-tokenized as a second `PI_TARGET` instead of `PI_TEXT`, because the
  single `pi` group offered `PI_TARGET`'s pattern for the whole PI body,
  not just the first token. The on-token callback now swaps from the `pi`
  group to a new `pi_body` group the instant `PI_TARGET` matches, so its
  pattern is never re-offered. Covered by a new regression test.

## [0.1.0] — 2026-03-29

### Added

- Initial implementation of `coding_adventures.xml_lexer`.
- `tokenize(source)` — tokenizes an XML string using the shared `xml.tokens`
  grammar and the grammar-driven `GrammarLexer` from `coding-adventures-lexer`.
- `get_grammar()` — returns the cached `TokenGrammar` for direct use.
- Group-switching `on_token` callback that drives context-sensitive lexing:
  pushes/pops pattern groups (`tag`, `comment`, `cdata`, `pi`) as structural
  tokens are emitted.
- Grammar is read from `code/grammars/xml.tokens` once and cached.
- Path navigation uses `debug.getinfo` to locate the grammar file relative to
  the installed module, avoiding hardcoded absolute paths.
- Comprehensive busted test suite covering all XML token types: opening/closing
  tags, self-closing tags, attributes (double- and single-quoted), text content,
  entity references, character references, comments, CDATA sections, processing
  instructions, and full-document round-trip.
- `required_capabilities.json` declaring `filesystem:read`.
- `BUILD` and `BUILD_windows` scripts with transitive dependency installation
  in leaf-to-root order.
