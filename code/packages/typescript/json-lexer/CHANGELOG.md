# Changelog

All notable changes to the JSON Lexer package will be documented in this file.

## [0.1.0] - 2026-03-20

### Added

- Initial release of the JSON lexer.
- `tokenizeJSON()` function that tokenizes JSON text using the grammar-driven lexer engine.
- Loads `json.tokens` grammar file defining STRING, NUMBER, TRUE, FALSE, NULL, and structural tokens.
- Full support for JSON number formats: integers, negatives, decimals, scientific notation.
- Full support for JSON string escape sequences: `\"`, `\\`, `\/`, `\b`, `\f`, `\n`, `\r`, `\t`, `\uXXXX`.
- Comprehensive test suite covering all JSON token types, nested structures, whitespace handling, position tracking, and error cases.
