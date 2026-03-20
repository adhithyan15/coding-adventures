# Changelog

All notable changes to the JSON Parser package will be documented in this file.

## [0.1.0] - 2026-03-20

### Added

- Initial release of the JSON parser.
- `parseJSON()` function that parses JSON text into ASTs using the grammar-driven parser engine.
- Loads `json.grammar` grammar file defining value, object, pair, and array rules.
- Full support for all JSON value types: strings, numbers, booleans, null, objects, and arrays.
- Support for arbitrarily deep nested structures via recursive grammar rules.
- Comprehensive test suite covering primitive values, objects, arrays, nested structures, whitespace tolerance, and error cases.
