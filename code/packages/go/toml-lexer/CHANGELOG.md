# Changelog

All notable changes to the toml-lexer package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- Initial implementation of the TOML lexer wrapper around the grammar-driven lexer
- `CreateTOMLLexer(source string)` factory function for creating reusable lexer instances
- `TokenizeTOML(source string)` convenience function for one-shot tokenization
- Grammar file loading via `runtime.Caller(0)` for location-independent operation
- Support for all TOML token types:
  - Four string types: basic, literal, multi-line basic, multi-line literal
  - Four date/time types: offset datetime, local datetime, local date, local time
  - Numbers: decimal, hex, octal, binary integers; decimal, scientific, special floats
  - Booleans: true, false
  - Bare keys: unquoted key names
  - Delimiters: = . , [ ] { }
  - Newlines (significant in TOML)
- Escape mode: none (quotes stripped, escapes left raw for parser semantic layer)
- Comprehensive test suite with 30+ test cases
