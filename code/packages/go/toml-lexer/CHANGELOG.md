# Changelog

All notable changes to the toml-lexer package will be documented in this file.

## [0.2.0] - 2026-07-13

### Changed

- The token grammar is now embedded at compile time as native Go data in
  `grammar_data.go` and consumed directly by the loader; the package no longer
  reads `code/grammars/**` at run time via `runtime.Caller`. This drops the
  filesystem capability (empty `required_capabilities.json`, `gen_capabilities.go`
  removed), lets the package build and run standalone, and fixes the previously
  dead embedded grammar (the old `_grammar.go` name is ignored by the Go
  toolchain because of its leading underscore).

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
