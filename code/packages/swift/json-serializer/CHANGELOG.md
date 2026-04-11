# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-04-06

### Added

- Initial implementation of `JsonSerializer` with compact and pretty modes
- `serialize(_:JsonValue) -> String`: converts any JsonValue to a JSON string
- `deserialize(_:String) throws -> JsonValue`: convenience wrapper around JsonParser
- Compact mode: no extra whitespace, minimal output (e.g., `{"a":1}`)
- Pretty mode: 2-space indentation per level, `: ` separator, empty containers on one line
- Full RFC 8259 string encoding: named escapes (`\n`, `\r`, `\t`, `\b`, `\f`, `\\`, `\"`) and `\uXXXX` for other control characters
- Integer-valued Doubles serialized without decimal point (`42.0` → `"42"`)
- Key insertion order preserved in object serialization
- `Sendable` conformance for Swift 6 concurrency
- Full test suite: compact/pretty serialization of all types, escape sequences, roundtrip tests
