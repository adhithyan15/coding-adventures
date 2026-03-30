# Changelog — coding-adventures-json-parser

## [0.1.0] - 2026-03-29

### Added

- Initial implementation of the grammar-driven JSON parser.
- `M.parse(source)` — tokenizes JSON source, loads `json.grammar`, runs
  `GrammarParser`, and returns the root `ASTNode`.
- `M.create_parser(source)` — returns an initialized `GrammarParser` for
  manual control (e.g., trace-mode debugging).
- `M.get_grammar()` — exposes the cached `ParserGrammar` for inspection.
- Grammar caching: `json.grammar` is read from disk and parsed exactly once
  per process; all subsequent calls reuse the cached grammar.
- Path navigation via `debug.getinfo` + `dirname`/`up` helpers, consistent
  with the pattern established by `json_lexer`.
- Full test suite (`tests/test_json_parser.lua`) covering:
  - Module surface (VERSION, parse, create_parser, get_grammar)
  - All scalar value types (string, number, true, false, null)
  - Empty object `{}` and empty array `[]`
  - Simple key-value objects and multi-pair objects
  - Arrays (single-element, multi-element, mixed types)
  - Nested structures (object in object, array in object, object in array,
    deeply nested `{"a": [1, 2, {"b": true}]}`)
  - Realistic JSON document
  - Error handling: trailing garbage, unterminated structures, missing colon,
    bare identifiers, empty input
- `BUILD` and `BUILD_windows` with transitive dependency installation in
  leaf-to-root order: state_machine → directed_graph → grammar_tools →
  lexer → json_lexer → parser → json_parser.
- `required_capabilities.json` declaring `filesystem:read`.
- `README.md` with architecture description, grammar listing, and usage examples.
