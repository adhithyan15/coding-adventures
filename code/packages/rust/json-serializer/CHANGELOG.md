# Changelog

## 0.1.0 (2026-03-22)

### Added
- `serialize()` for compact JSON output (no whitespace)
- `serialize_pretty()` for human-readable JSON with configurable formatting
- `SerializerConfig` with indent_size, indent_char, sort_keys, trailing_newline
- RFC 8259 compliant string escaping (quotes, backslash, control characters)
- Error handling for non-finite floats (Infinity, NaN)
- Comprehensive test suite (62 tests covering serialization, escaping, pretty-printing, round-trips)
