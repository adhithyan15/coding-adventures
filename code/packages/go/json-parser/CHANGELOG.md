# Changelog

All notable changes to the json-parser package will be documented in this file.

## [0.1.0] - 2026-03-20

### Added
- Initial implementation of the JSON parser wrapper around the grammar-driven parser
- `CreateJSONParser(source string)` factory function for creating reusable parser instances
- `ParseJSON(source string)` convenience function for one-shot parsing
- Grammar file loading via `runtime.Caller(0)` for location-independent operation
- Two-stage pipeline: json-lexer tokenization followed by grammar-driven recursive descent parsing
- Comprehensive test suite covering:
  - Standalone values: strings, numbers, literals (true, false, null)
  - Empty objects and arrays
  - Simple and multi-key objects
  - Simple, mixed-type, and single-element arrays
  - Nested objects, nested arrays, and mixed nesting
  - Object with array values and array of objects
  - Deeply nested structures (4 levels)
  - Multi-line (pretty-printed) JSON
  - Negative numbers in various contexts
  - Complex realistic JSON documents
  - All seven JSON value types in a single document
  - Factory function (CreateJSONParser) API
