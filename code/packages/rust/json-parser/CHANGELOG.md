# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-07-06

### Added

- `try_parse_json()` — like `parse_json()` but returns `Result<GrammarASTNode, String>`
  instead of panicking. Both the tokenize and parse steps are fallible here (via
  `json-lexer`'s new `try_tokenize_json`), so a malformed **untrusted** document
  (a file, a request body) is an error to handle rather than a crash. The
  panicking `parse_json()` remains for pre-validated input.
- 2 tests: valid input returns `Ok`; unterminated / stray-token / empty inputs
  return `Err` (never panic).

## [0.1.0] - 2026-03-20

### Added

- Initial release of the JSON parser crate.
- `create_json_parser()` factory function returning a `GrammarParser` configured for JSON.
- `parse_json()` convenience function returning a `GrammarASTNode` directly.
- Loads the `json.grammar` file at runtime from the shared `grammars/` directory.
- Depends on `coding-adventures-json-lexer` for tokenization.
- Supports all JSON grammar rules: value, object, pair, array.
- 16 unit tests covering: simple values (number, string, true/false/null), empty containers, objects (single pair, multi pair), arrays (simple, mixed type), nested structures (nested object, nested array, deeply nested), whitespace handling, complex numbers in context, escaped strings, and the factory function.
