# Changelog

All notable changes to this project will be documented in this file.

## [0.1.1] - 2026-03-31

### Fixed

- **JSON string escape sequences now decoded correctly**: The JSON grammar uses
  `escapes: none`, which means the lexer returns STRING tokens with raw escape
  sequences (e.g. `\n` as two characters). Previously `from_ast()` passed the
  raw token value directly to `JsonValue::String`, so `"hello\nworld"` would
  produce a string containing a literal backslash-n instead of a real newline.
  Added `unescape_json_string()` helper that decodes all JSON escape sequences
  (`\"`, `\\`, `\/`, `\b`, `\f`, `\n`, `\r`, `\t`, `\uXXXX`) before
  constructing the `JsonValue::String`. Test `test_from_ast_string_with_escapes`
  now passes. The fix also applies to object keys in `convert_pair_node`.

## [0.1.0] - 2026-03-22

### Added

- `JsonValue::Object` -- ordered collection of key-value pairs
- `JsonValue::Array` -- ordered sequence of values
- `JsonValue::String` -- text value wrapper
- `JsonValue::Number` -- numeric value (integer or float) with `integer?` predicate
- `JsonValue::Boolean` -- true/false wrapper
- `JsonValue::Null` -- null representation
- `JsonValue::Error` -- exception class for conversion errors
- `JsonValue.from_ast(ast)` -- converts json-parser AST to JsonValue tree
- `JsonValue.to_native(value)` -- converts JsonValue to native Ruby types
- `JsonValue.from_native(value)` -- converts native Ruby types to JsonValue
- `JsonValue.parse(text)` -- parses JSON text into JsonValue
- `JsonValue.parse_native(text)` -- parses JSON text into native Ruby types
