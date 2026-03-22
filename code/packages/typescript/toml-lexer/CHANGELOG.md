# Changelog

All notable changes to the TOML Lexer package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- Initial release of the TOML lexer.
- `tokenizeTOML()` function that tokenizes TOML text using the grammar-driven lexer engine.
- Loads `toml.tokens` grammar file defining all TOML v1.0.0 token types.
- Four string types: basic, literal, multi-line basic, multi-line literal.
- Date/time literals: offset datetime, local datetime, local date, local time.
- Multiple integer formats: decimal, hexadecimal (0x), octal (0o), binary (0b).
- Float formats: decimal, scientific notation, special values (inf, nan).
- Bare key recognition for unquoted TOML key names.
- Newline-sensitive tokenization (NEWLINE tokens emitted between lines).
- Comment skipping (# to end of line).
- `escapes: none` support for deferring escape processing to the semantic layer.
- Comprehensive test suite covering all TOML token types, ordering, newlines, comments, and position tracking.
