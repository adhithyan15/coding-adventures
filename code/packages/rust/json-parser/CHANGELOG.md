# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-20

### Added

- Initial release of the JSON parser crate.
- `create_json_parser()` factory function returning a `GrammarParser` configured for JSON.
- `parse_json()` convenience function returning a `GrammarASTNode` directly.
- Loads the `json.grammar` file at runtime from the shared `grammars/` directory.
- Depends on `coding-adventures-json-lexer` for tokenization.
- Supports all JSON grammar rules: value, object, pair, array.
- 16 unit tests covering: simple values (number, string, true/false/null), empty containers, objects (single pair, multi pair), arrays (simple, mixed type), nested structures (nested object, nested array, deeply nested), whitespace handling, complex numbers in context, escaped strings, and the factory function.
