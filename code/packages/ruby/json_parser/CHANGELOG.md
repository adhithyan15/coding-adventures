# Changelog

All notable changes to `coding_adventures_json_parser` will be documented in this file.

## [0.1.0] - 2026-03-20

### Added
- Initial release
- `CodingAdventures::JsonParser.parse(source)` method that parses JSON text into ASTs
- Loads `json.grammar` and delegates to `GrammarDrivenParser`
- Supports all JSON value types: strings, numbers, booleans (true/false), null
- Supports empty and non-empty objects with key-value pairs
- Supports empty and non-empty arrays with mixed-type elements
- Supports arbitrarily deep nested structures (objects in arrays, arrays in objects)
- Error handling for invalid JSON (trailing commas, missing colons, empty input)
- Full test suite with SimpleCov coverage >= 80%
