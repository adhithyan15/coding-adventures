# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-22

### Added

- `JsonValue` discriminated union type with six variants: object, array, string, number, boolean, null
- Factory functions: `jsonObject`, `jsonArray`, `jsonString`, `jsonNumber`, `jsonBool`, `jsonNull`
- `fromAST(node)` -- convert json-parser ASTNode trees to typed JsonValue trees
- `toNative(value)` -- convert JsonValue to plain JavaScript types (object, array, string, number, boolean, null)
- `fromNative(value)` -- convert native JavaScript types to JsonValue (with error handling for non-JSON types)
- `parse(text)` -- convenience function: JSON text to JsonValue
- `parseNative(text)` -- convenience function: JSON text to native JS types
- `JsonValueError` custom error class for all failure modes
- Full JSON escape sequence handling including Unicode surrogate pairs
- Integer vs float distinction for JSON numbers (based on decimal point and exponent presence)
- Comprehensive test suite with 95%+ coverage
