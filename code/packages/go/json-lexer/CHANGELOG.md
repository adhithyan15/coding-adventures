# Changelog

All notable changes to the json-lexer package will be documented in this file.

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
