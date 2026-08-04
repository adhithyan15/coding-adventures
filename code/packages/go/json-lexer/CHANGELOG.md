# Changelog

All notable changes to the json-lexer package will be documented in this file.

## [0.2.0] - 2026-07-13

### Changed

- The token grammar is now embedded at compile time as native Go data in
  `grammar_data.go` and consumed directly by the loader; the package no longer
  reads `code/grammars/**` at run time via `runtime.Caller`. This drops the
  filesystem capability (empty `required_capabilities.json`, `gen_capabilities.go`
  removed), lets the package build and run standalone, and fixes the previously
  dead embedded grammar (the old `_grammar.go` name is ignored by the Go
  toolchain because of its leading underscore).

## [0.1.1] - 2026-03-31

### Fixed

- `TestTokenizeJSONStringWithEscapes` now uses a local `escapeProcessingGrammarSrc`
  helper (identical to `json.tokens` but without `escapes: none`) instead of the
  real JSON grammar. The real JSON grammar intentionally leaves escape sequences
  raw for the parser to decode; the test was incorrectly expecting the lexer to
  process them. The test now correctly targets the lexer engine's escape
  processing capability while leaving the JSON grammar semantics unchanged.

## [0.1.0] - 2026-03-20

### Added
- Initial implementation of the JSON lexer wrapper around the grammar-driven lexer
- `CreateJSONLexer(source string)` factory function for creating reusable lexer instances
- `TokenizeJSON(source string)` convenience function for one-shot tokenization
- Grammar file loading via `runtime.Caller(0)` for location-independent operation
- Support for all JSON token types: STRING, NUMBER, TRUE, FALSE, NULL, and structural tokens
- Comprehensive test suite covering:
  - Simple and escaped strings
  - Empty strings
  - Integer, negative, decimal, and exponent numbers
  - Literal values (true, false, null)
  - All six structural tokens ({ } [ ] : ,)
  - Whitespace skipping
  - Simple and complex objects
  - Simple and nested arrays
  - Empty objects and arrays
  - Multi-line (pretty-printed) JSON
  - EOF token verification
  - Line and column tracking
