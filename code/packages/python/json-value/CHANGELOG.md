# Changelog

All notable changes to the JSON value package will be documented in this file.

## [0.1.0] - 2026-03-22

### Added
- Initial release of the JSON value package.
- `JsonValue` base class and six concrete subclasses: `JsonObject`, `JsonArray`, `JsonString`, `JsonNumber`, `JsonBool`, `JsonNull`.
- `from_ast()` function to convert json-parser ASTs to JsonValue trees.
- `to_native()` function to convert JsonValue trees to native Python types (dict, list, str, int, float, bool, None).
- `from_native()` function to convert native Python types to JsonValue trees.
- `parse()` convenience function: JSON text to JsonValue in one call.
- `parse_native()` convenience function: JSON text to native Python types in one call.
- `JsonValueError` exception for all error conditions.
- Comprehensive test suite with 44+ tests covering all spec requirements.
