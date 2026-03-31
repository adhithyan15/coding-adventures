# Changelog — CodingAdventures::JsonParser

## [0.01] - 2026-03-29

### Added

- Initial implementation of the hand-written recursive-descent JSON parser.
- `CodingAdventures::JsonParser->parse($source)` — tokenizes JSON source
  with `CodingAdventures::JsonLexer`, then applies four recursive-descent
  productions (value, object, pair, array) to build an AST.
- `CodingAdventures::JsonParser::ASTNode` — blessed-hashref AST node with
  accessors `rule_name`, `children`, `is_leaf`, `token`.
- Internal helpers: `_peek`, `_advance`, `_expect`, `_node`, `_leaf`.
- Descriptive die messages including token type, value, line, and column.
- Full test suite (`t/00-load.t`, `t/01-basic.t`) covering:
  - Module load verification for both JsonParser and ASTNode
  - Root node has rule_name "value"
  - All scalar value types (string, number, true, false, null)
  - Empty object `{}` and empty array `[]`
  - Simple key-value object with leaf inspection
  - Object with multiple pairs
  - Array of numbers, strings, mixed types
  - Nested structures (object-in-object, array-in-object, object-in-array)
  - Deeply nested `{"a": [1, 2, {"b": true}]}`
  - Realistic JSON document
  - ASTNode accessor tests (new, rule_name, is_leaf, token, children)
  - Error cases: trailing garbage, unterminated object/array, missing colon,
    bare identifier, empty input
- `BUILD` and `BUILD_windows` with transitive dependency installation in
  leaf-to-root order: state-machine → directed-graph → grammar-tools →
  lexer → json-lexer → json-parser.
- `cpanfile` and `Makefile.PL`.
- `README.md` explaining the hand-written approach, grammar, usage, and API.
