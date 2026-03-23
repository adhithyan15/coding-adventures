# Changelog

All notable changes to this project will be documented in this file.

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
