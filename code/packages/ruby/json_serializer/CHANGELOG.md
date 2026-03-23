# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-22

### Added

- `JsonSerializer.serialize(value)` -- compact JSON output from JsonValue
- `JsonSerializer.serialize_pretty(value, config:)` -- pretty-printed JSON with configurable formatting
- `JsonSerializer.stringify(value)` -- compact JSON from native Ruby types
- `JsonSerializer.stringify_pretty(value, config:)` -- pretty JSON from native Ruby types
- `SerializerConfig` -- configuration for indent size, indent char, key sorting, trailing newline
- `JsonSerializer::Error` -- exception for non-serializable values (Infinity, NaN)
- RFC 8259 compliant string escaping (quotes, backslash, control characters)
