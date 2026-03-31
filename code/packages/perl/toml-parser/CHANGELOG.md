# Changelog — CodingAdventures::TomlParser

## [0.01] - 2026-03-29

### Added

- Initial implementation of the hand-written recursive-descent TOML parser.
- `CodingAdventures::TomlParser->parse($source)` — tokenizes TOML source
  with `CodingAdventures::TomlLexer`, then applies recursive-descent
  productions to build an AST with `rule_name == "document"` at the root.
- `CodingAdventures::TomlParser::ASTNode` — blessed-hashref AST node with
  accessors `rule_name`, `children`, `is_leaf`, `token`.
- Productions implemented: `_parse_document`, `_parse_expression`,
  `_parse_keyval`, `_parse_key`, `_parse_simple_key`, `_parse_table_header`,
  `_parse_array_table_header`, `_parse_value`, `_parse_array`,
  `_parse_array_values`, `_parse_inline_table`.
- `%SIMPLE_KEY_TYPES` and `%SCALAR_VALUE_TYPES` lookup tables for efficient
  token-type dispatch.
- `_peek_at($offset)` helper for one-token lookahead (used to distinguish
  `[[array]]` from `[table]`).
- Full NEWLINE handling: blank lines are consumed as NEWLINE leaf nodes in
  `document`; newlines inside arrays are consumed in `array_values`.
- Descriptive die messages including token type, value, line, and column.
- Full test suite (`t/00-load.t`, `t/01-basic.t`) covering:
  - Module load verification for both TomlParser and ASTNode
  - Root node has rule_name "document"
  - Key-value pairs: strings, integers, floats, booleans
  - Multiple key-value pairs
  - Bare keys, quoted keys, dotted keys (a.b, a.b.c)
  - Table headers [section] and dotted headers [a.b]
  - Multiple table sections
  - Array-of-tables headers [[products]]
  - Multiple [[array]] headers
  - Inline arrays (empty, integers, strings, nested)
  - Inline tables (empty, single pair, multi-pair)
  - Realistic multi-section TOML document
  - ASTNode accessor tests
  - Error cases: missing equals, unterminated array, unterminated header
- `BUILD` and `BUILD_windows` with transitive dependency installation in
  leaf-to-root order: state-machine → directed-graph → grammar-tools →
  lexer → toml-lexer → toml-parser.
- `cpanfile` and `Makefile.PL`.
- `README.md` explaining the hand-written approach, grammar, TOML-specific
  considerations, usage, and error handling.
