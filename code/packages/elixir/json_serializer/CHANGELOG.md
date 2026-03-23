# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-22

### Added
- `serialize/1` — compact JSON output from typed JSON values
- `serialize_pretty/2` — pretty-printed JSON with configurable indentation
- `stringify/1` — compact JSON from native Elixir types
- `stringify_pretty/2` — pretty-printed JSON from native Elixir types
- RFC 8259 string escaping (quotes, backslash, control characters, \uXXXX)
- Configuration options: indent_size, indent_char, sort_keys, trailing_newline
- Support for all six JSON types: object, array, string, number, boolean, null
