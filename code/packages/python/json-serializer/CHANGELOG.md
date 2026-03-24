# Changelog

All notable changes to the `json-serializer` package will be documented here.

## [0.1.0] - 2026-03-22

### Added

- `serialize(value: JsonValue) -> str` for compact JSON output
- `serialize_pretty(value: JsonValue, config?) -> str` for pretty-printed output
- `stringify(value: native) -> str` convenience function (native -> compact JSON)
- `stringify_pretty(value: native, config?) -> str` convenience function (native -> pretty JSON)
- `SerializerConfig` dataclass with `indent_size`, `indent_char`, `sort_keys`, `trailing_newline`
- `JsonSerializerError` exception for non-serializable values (Infinity, NaN)
- Full RFC 8259 string escaping (quotes, backslash, control characters, \uXXXX)
- Literate programming style with extensive inline documentation
