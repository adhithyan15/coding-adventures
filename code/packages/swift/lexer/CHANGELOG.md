# Changelog

All notable changes to the Lexer package will be documented in this file.

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
