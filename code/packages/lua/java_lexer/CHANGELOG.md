# Changelog — coding-adventures-java-lexer (Lua)

All notable changes to this package are documented here.

## [0.1.0] — 2026-04-11

### Added

- Initial implementation of `coding_adventures.java_lexer`.
- `tokenize(source, version)` — tokenizes a Java string using the shared
  `java/java<version>.tokens` grammar and the grammar-driven `GrammarLexer` from
  `coding-adventures-lexer`.
- `create_lexer(source, version)` — returns an initialized `GrammarLexer`
  without immediately tokenizing.
- `get_grammar(version)` — returns the cached `TokenGrammar` for direct use.
- Version routing: when `version` is `"1.0"`, `"1.1"`, `"1.4"`, `"5"`,
  `"7"`, `"8"`, `"10"`, `"14"`, `"17"`, or `"21"`, the corresponding
  versioned grammar file is loaded from `code/grammars/java/java<version>.tokens`.
- Default version: passing `nil` or `""` defaults to Java 21.
- Per-version grammar cache keyed by version string.
- `\v` and `\f` escape normalization applied when loading grammar files
  (Lua's regex engine requires literal control characters instead).
- Validation: unknown version strings raise a descriptive error immediately.
- Comprehensive busted test suite covering module surface, empty/trivial
  inputs, identifiers, numbers, strings, punctuation, whitespace handling,
  position tracking, version-aware tokenization, cache behavior, and error
  cases.
- `required_capabilities.json` declaring `filesystem:read`.
- `BUILD` and `BUILD_windows` scripts with transitive dependency installation.
