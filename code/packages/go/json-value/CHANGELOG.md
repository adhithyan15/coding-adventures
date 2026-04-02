# Changelog

## [0.1.2] - 2026-04-02

### Changed

- Wrapped all public functions (`FromAST`, `ToNative`, `FromNative`, `Parse`,
  `ParseNative`) with the Operations system via `StartNew[T]`. Public API
  signatures are unchanged.

## [0.1.1] - 2026-03-31

### Fixed

- **JSON string escape sequences now decoded correctly**: The JSON grammar uses
  `escapes: none`, which means the lexer returns STRING tokens with raw escape
  sequences (e.g. `\n` as two characters). Previously `FromAST()` passed the
  raw token value directly to `JsonString`, so `"hello\nworld"` would produce
  `JsonString{Value: "hello\\nworld"}` instead of `JsonString{Value: "hello\nworld"}`.
  Added `unescapeJSONString()` helper that decodes all JSON escape sequences
  (`\"`, `\\`, `\/`, `\b`, `\f`, `\n`, `\r`, `\t`, `\uXXXX`) before
  constructing the `JsonString`. Test `TestParseStringWithEscapes` now passes.

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
