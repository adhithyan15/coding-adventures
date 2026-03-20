# Changelog

All notable changes to the JSON parser package will be documented in this file.

## [0.1.0] - 2026-03-20

### Added
- Initial release of the JSON parser thin wrapper.
- `parse_json()` function for one-step parsing of JSON text into ASTs.
- `create_json_parser()` factory for creating configured `GrammarParser` instances.
- Full RFC 8259 grammar support: objects, arrays, strings, numbers, booleans, null.
- Produces generic `ASTNode` trees — the same type used for all grammar-driven languages.
