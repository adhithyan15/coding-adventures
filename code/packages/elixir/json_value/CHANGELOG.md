# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-22

### Added
- `from_ast/1` — convert parser AST nodes to typed JSON values (tagged tuples)
- `to_native/1` — convert JSON values to native Elixir types (map, list, etc.)
- `from_native/1` — convert native Elixir types to JSON values
- `parse/1` — end-to-end JSON text to typed value parsing
- `parse_native/1` — end-to-end JSON text to native Elixir type parsing
- Support for all six JSON types: object, array, string, number, boolean, null
- Integer/float distinction for numbers (42 is integer, 3.14 is float)
- Ordered pairs for objects (preserves insertion order)
- Comprehensive error handling for invalid inputs
