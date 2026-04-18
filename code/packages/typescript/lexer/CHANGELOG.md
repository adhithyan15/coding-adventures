# Changelog

All notable changes to this package will be documented in this file.

## [0.3.0] - 2026-04-18

### Added

- Optional rich source preservation for `GrammarLexer` and `grammarTokenize()`
  via `{ preserveSourceInfo: true }`.
- `Trivia` type for preserved skip matches such as whitespace and comments.
- Optional token metadata fields:
  - `startOffset` / `endOffset`
  - `endLine` / `endColumn`
  - `tokenIndex`
  - `leadingTrivia`
- `typeName?: string` on `Token` so layout-mode virtual tokens and similar
  source-preserving callers can remain type-safe.

## [0.2.0] - 2026-03-21

### Added

- `GrammarLexer` class — class-based grammar-driven lexer with support for
  pattern groups and on-token callbacks. Replaces the internal implementation
  of `grammarTokenize` while maintaining backward compatibility.
- `LexerContext` class — callback interface for controlling the lexer during
  tokenization. Provides methods for:
  - `pushGroup(groupName)` / `popGroup()` — switch between pattern groups
  - `activeGroup()` / `groupStackDepth()` — inspect group stack state
  - `emit(token)` — inject synthetic tokens after the current one
  - `suppress()` — suppress the current token from output
  - `peek(offset)` / `peekStr(length)` — lookahead into source text
  - `setSkipEnabled(enabled)` — toggle skip pattern processing
- `OnTokenCallback` type — signature for on-token callback functions.
- `GrammarLexer.setOnToken(callback)` — register a callback that fires on
  every token match (except skip matches, emitted tokens, and EOF).
- Pattern group support in `GrammarLexer` — compiles and uses per-group
  patterns from the grammar's `groups` field. The group stack starts at
  "default" and resets between `tokenize()` calls.
- Comprehensive test suite for `LexerContext` (10 unit tests) and pattern
  group tokenization (13 integration tests) covering push/pop, suppress,
  emit, token replacement, skip toggling, nested structures, and backward
  compatibility.
- Exported `GrammarLexer`, `LexerContext`, and `OnTokenCallback` from
  package index.

### Changed

- `grammarTokenize()` is now a thin wrapper around `GrammarLexer.tokenize()`.
  All existing callers continue to work without changes.

## [0.1.0] - 2026-03-19

### Added

- Initial TypeScript port of the lexer package from Python.
- `Token` interface with `type`, `value`, `line`, `column` fields.
- `tokenize()` — hand-written character-by-character lexer supporting:
  - Integer literals (NUMBER)
  - Identifiers (NAME)
  - String literals with escape sequences (STRING)
  - Configurable keyword recognition (KEYWORD)
  - Operators: `+`, `-`, `*`, `/`, `=`, `==`
  - Delimiters: `(`, `)`, `,`, `:`
  - Newline tokens and EOF sentinel
  - Position tracking (line and column numbers)
  - Error reporting with `LexerError`
- `grammarTokenize()` — grammar-driven lexer that reads token definitions from a `TokenGrammar` object (parsed from `.tokens` files by `@coding-adventures/grammar-tools`).
  - Regex and literal pattern compilation
  - First-match-wins priority ordering
  - Keyword detection via grammar keyword lists
  - String escape sequence processing
  - Full interchangeability with `tokenize()`
- Comprehensive test suite for both lexer implementations.
- Comparison tests verifying both lexers produce identical output.
- Custom grammar tests for programmatically-built grammars.
- Ruby grammar integration tests.
