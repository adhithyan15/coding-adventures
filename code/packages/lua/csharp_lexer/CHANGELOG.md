# Changelog — coding-adventures-csharp-lexer (Lua)

All notable changes to this package are documented here.

## [0.1.0] — 2026-04-11

### Added

- Initial implementation of `coding_adventures.csharp_lexer`.
- `tokenize_csharp(source, version)` — tokenizes a C# string using the shared
  `csharp/csharp<version>.tokens` grammar and the grammar-driven `GrammarLexer` from
  `coding-adventures-lexer`.
- `create_csharp_lexer(source, version)` — returns an initialized `GrammarLexer`
  without immediately tokenizing.
- `get_grammar(version)` — returns the cached `TokenGrammar` for direct use.
- Version routing: when `version` is `"1.0"`, `"2.0"`, `"3.0"`, `"4.0"`,
  `"5.0"`, `"6.0"`, `"7.0"`, `"8.0"`, `"9.0"`, `"10.0"`, `"11.0"`, or `"12.0"`,
  the corresponding versioned grammar file is loaded from
  `code/grammars/csharp/csharp<version>.tokens`.
- Default version: passing `nil` or `""` defaults to C# 12.0.
- Per-version grammar cache keyed by version string.
- `\v` and `\f` escape normalization applied when loading grammar files
  (Lua's regex engine requires literal control characters instead).
- Validation: unknown version strings raise a descriptive error immediately.
- Comprehensive busted test suite covering module surface, empty/trivial
  inputs, identifiers, numbers, strings, punctuation, whitespace handling,
  position tracking, version-aware tokenization (all 12 versions), cache
  behavior, and error cases.
- `required_capabilities.json` declaring `filesystem:read`.
- `BUILD` and `BUILD_windows` scripts with transitive dependency installation.
