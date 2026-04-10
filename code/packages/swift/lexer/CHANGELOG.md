# Changelog

All notable changes to the Lexer package will be documented in this file.

## [0.1.1] - 2026-04-10

### Fixed
- `GrammarLexer` now correctly lowercases the source when `case_sensitive: false`
  is set, even when `@case_insensitive true` is also present in the grammar. Previously
  the two directives combined would prevent source lowercasing, causing uppercase input
  like "LET" to fail matching against lowercase patterns like `/[a-z][a-z0-9]*/`.
  Source lowercasing is now driven solely by `case_sensitive: false`; the `@case_insensitive`
  flag continues to control per-token keyword-promotion behaviour only.

## [0.1.0] - 2026-04-04

### Added
- Initial Swift port of the TypeScript grammar-driven lexer
- `Token` struct with type, value, line, column, and flags fields
- `LexerError` error type with message, line, and column
- `GrammarLexer` class with full feature set:
  - Pattern compilation from `TokenGrammar`
  - First-match-wins tokenization
  - Pattern groups with stack-based activation
  - `OnTokenCallback` with `LexerContext`
  - Pre/post tokenize hooks
  - Indentation mode with INDENT/DEDENT
  - `_lastEmittedToken` tracking for `previousToken()`
  - `_bracketDepths` tracking for `bracketDepth()`
  - `_contextKeywordSet` for context keywords with `TOKEN_CONTEXT_KEYWORD` flag
  - `precededByNewline()` on `LexerContext`
- `LexerContext` class with:
  - `pushGroup()`, `popGroup()`, `activeGroup()`, `groupStackDepth()`
  - `emit()`, `suppress()`
  - `peek()`, `peekStr()`
  - `setSkipEnabled()`
  - `previousToken()` for lookbehind
  - `bracketDepth(kind:)` for nesting tracking
  - `precededByNewline()` for newline detection
- `grammarTokenize(source:grammar:)` convenience function
- `TOKEN_PRECEDED_BY_NEWLINE` and `TOKEN_CONTEXT_KEYWORD` flag constants
- `BracketKind` enum (paren, bracket, brace)
- Comprehensive test suite covering all features
