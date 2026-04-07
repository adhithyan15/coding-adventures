# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-04-06

### Added

- Initial implementation of `JsonLexer` tokenizing JSON text to `[Token]`
- `TokenKind` enum with 11 cases: 6 structural punctuation, `stringLit`, `numberLit`, `trueLit`, `falseLit`, `nullLit`
- `Token` struct with `kind` and `offset` (byte position in source)
- `JsonLexError` with descriptive message and source offset
- Full RFC 8259 string escape support: `\"`, `\\`, `\/`, `\b`, `\f`, `\n`, `\r`, `\t`, `\uXXXX`
- Surrogate pair handling for Unicode code points above U+FFFF
- Number lexing: integer, decimal, scientific notation (e/E, +/- exponent)
- Rejects leading zeros (`007`), trailing decimal (`.`), bare minus
- Rejects raw control characters in strings (must be `\uXXXX` escaped)
- `Sendable` conformance on all public types for Swift 6 concurrency
- Full test suite covering all token types, escapes, numbers, and error cases
