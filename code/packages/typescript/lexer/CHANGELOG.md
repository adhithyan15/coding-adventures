# Changelog

All notable changes to this package will be documented in this file.

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
