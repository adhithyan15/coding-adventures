# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-04-06

### Added

- Initial implementation of `JsonValue` enum with six cases: `null`, `bool`, `number`, `string`, `array`, `object`
- `Equatable` conformance with structural equality for all cases including ordered object pairs
- `CustomStringConvertible` with compact JSON-like output (integers render without decimal point)
- Convenience accessors: `stringValue`, `doubleValue`, `boolValue`, `arrayValue`, `objectValue`, `isNull`
- Subscript access: `value["key"]` for objects, `value[index]` for arrays, with Optional chaining support
- `JsonValue.from(_:Bool)` factory method to avoid ambiguity with integer literals
- `Sendable` conformance for safe use across Swift concurrency domains
- Full test suite with 100% case coverage: construction, equality, subscripts, description
