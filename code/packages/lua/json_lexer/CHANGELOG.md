# Changelog — coding-adventures-json-lexer (Lua)

All notable changes to this package are documented here.

## [Unreleased]

### Changed

- Bundle a byte-locked Lua projection of the canonical `json.tokens` fixture
  so installed rocks no longer depend on a checkout-relative grammar file.
- Replace the ambient filesystem capability with an empty authority profile.
- Add a drift test that keeps the deployed payload aligned with the canonical
  language-neutral grammar.

## [0.1.0] — 2026-03-29

### Added

- Initial implementation of `coding_adventures.json_lexer`.
- `tokenize(source)` — tokenizes a JSON string using the shared `json.tokens`
  grammar and the grammar-driven `GrammarLexer` from `coding-adventures-lexer`.
- `get_grammar()` — returns the cached `TokenGrammar` for direct use.
- Grammar is read from `code/grammars/json.tokens` once and cached.
- Path navigation uses `debug.getinfo` to locate the grammar file relative to
  the installed module, avoiding hardcoded absolute paths.
- Comprehensive busted test suite covering all token types, escape sequences,
  whitespace handling, position tracking, nested structures, and error cases.
- `required_capabilities.json` declaring `filesystem:read` (reads grammar file
  at startup).
- `BUILD` and `BUILD_windows` scripts with transitive dependency installation
  in leaf-to-root order.
