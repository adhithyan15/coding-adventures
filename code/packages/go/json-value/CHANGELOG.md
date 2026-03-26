# Changelog

## 0.1.0 — 2026-03-22

### Added

- `JsonValue` interface with concrete types: `JsonObject`, `JsonArray`, `JsonString`, `JsonNumber`, `JsonBool`, `JsonNull`
- `KeyValuePair` struct for ordered object representation
- `FromAST()` -- convert json-parser AST nodes to JsonValue types
- `ToNative()` -- convert JsonValue to native Go types (map, slice, string, etc.)
- `FromNative()` -- convert native Go types to JsonValue
- `Parse()` -- convenience function: JSON text to JsonValue
- `ParseNative()` -- convenience function: JSON text to native Go types
- `JsonValueError` error type for conversion failures
- Integer vs float number distinction via `IsInteger` flag
- Support for all Go integer types (int, int8-64, uint, uint8-64) in `FromNative`
- Comprehensive test suite with 40+ test cases
