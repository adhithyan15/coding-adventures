# Changelog

All notable changes to the toml-parser package will be documented in this file.

## [0.2.0] - 2026-07-13

### Changed

- The parser grammar is now embedded at compile time as native Go data in
  `grammar_data.go` and consumed directly by the loader; the package no longer
  reads `code/grammars/**` at run time via `runtime.Caller`. This drops the
  filesystem capability (empty `required_capabilities.json`, `gen_capabilities.go`
  removed), lets the package build and run standalone, and fixes the previously
  dead embedded grammar (the old `_grammar.go` name is ignored by the Go
  toolchain because of its leading underscore).

## [0.1.0] - 2026-03-21

### Added
- Initial implementation of the TOML parser wrapper around the grammar-driven parser
- `CreateTOMLParser(source string)` factory function for creating reusable parser instances
- `ParseTOML(source string)` convenience function for one-shot parsing
- Grammar file loading via `runtime.Caller(0)` for location-independent operation
- Two-stage pipeline: toml-lexer tokenization followed by grammar-driven recursive descent parsing
- Comprehensive test suite covering:
  - Key-value pairs with all value types
  - Table headers and array-of-tables headers
  - Dotted keys
  - Arrays (single-line and multi-line)
  - Inline tables
  - All four string types
  - All four date/time types
  - Numbers (integers, floats, hex, octal, binary, special floats)
  - Booleans
  - Complex realistic TOML documents
  - Factory function (CreateTOMLParser) API
