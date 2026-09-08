# Changelog

## Unreleased

- Re-exported the canonical `bounded-json` value and number types so existing
  consumers remain source-compatible while security-sensitive storage paths
  migrate directly to the bounded parser and serializer.

## 0.1.0 (2026-03-22)

### Added
- `JsonValue` enum with six variants: Object, Array, String, Number, Bool, Null
- `JsonNumber` enum distinguishing Integer (i64) from Float (f64)
- `from_ast()` function to convert json-parser AST nodes to JsonValue
- `parse()` convenience function for text-to-JsonValue conversion
- `JsonValueError` error type with descriptive messages
- Comprehensive test suite (40 tests covering all JSON types, nesting, escapes, errors)
