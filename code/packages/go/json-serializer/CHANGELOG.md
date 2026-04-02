# Changelog

## 0.1.1 — 2026-04-02

### Changed

- Wrapped all public functions (`DefaultConfig`, `Serialize`, `SerializePretty`,
  `Stringify`, `StringifyPretty`) with the Operations system via `StartNew[T]`.
  Public API signatures are unchanged.

## 0.1.0 — 2026-03-22

### Added

- `Serialize()` -- compact JSON output from JsonValue types
- `SerializePretty()` -- pretty-printed JSON with configurable indentation
- `Stringify()` -- convenience: native Go types to compact JSON
- `StringifyPretty()` -- convenience: native Go types to pretty JSON
- `SerializerConfig` struct with IndentSize, IndentChar, SortKeys, TrailingNewline
- `DefaultConfig()` factory function
- RFC 8259 compliant string escaping (quotes, backslash, control characters)
- Error handling for non-serializable values (Infinity, NaN)
- `JsonSerializerError` error type
- Comprehensive test suite with 60+ test cases including round-trip tests
