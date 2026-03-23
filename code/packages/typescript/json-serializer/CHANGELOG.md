# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-22

### Added

- `serialize(value)` -- compact JSON serialization from JsonValue
- `serializePretty(value, config?)` -- pretty-printed JSON with configurable indentation
- `stringify(native)` -- compact JSON from native JS types
- `stringifyPretty(native, config?)` -- pretty-printed JSON from native JS types
- `SerializerConfig` interface with indentSize, indentChar, sortKeys, trailingNewline options
- `JsonSerializerError` for Infinity/NaN serialization errors
- Full RFC 8259 string escaping (quotes, backslash, control characters, \uXXXX)
- Comprehensive test suite with 95%+ coverage
