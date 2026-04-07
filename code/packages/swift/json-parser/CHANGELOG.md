# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-04-06

### Added

- Initial implementation of `JsonParser` using recursive-descent parsing
- `parse(_:String)` convenience method combining lexing + parsing in one call
- `parseTokens(_:[Token])` for parsing pre-lexed token arrays
- Full support for all six JSON value types: null, bool, number, string, array, object
- `parseArray` handles empty arrays, single elements, and comma-separated lists
- `parseObject` handles empty objects, single pairs, and comma-separated key-value pairs
- Insertion order preserved in object pairs (uses `[(key:value:)]` not `Dictionary`)
- Strict rejection of trailing commas (not valid JSON)
- Strict requirement for string keys in objects
- `JsonParseError` with descriptive messages for all error conditions
- `Sendable` conformance for Swift 6 concurrency
- Full test suite covering primitives, arrays, objects, complex nested structures, and all error cases
