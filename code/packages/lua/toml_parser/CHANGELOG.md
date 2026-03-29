# Changelog — coding-adventures-toml-parser

## [0.1.0] - 2026-03-29

### Added

- Initial implementation of the grammar-driven TOML parser.
- `M.parse(source)` — tokenizes TOML source, loads `toml.grammar`, runs
  `GrammarParser`, and returns the root `ASTNode` with `rule_name == "document"`.
- `M.create_parser(source)` — returns an initialized `GrammarParser` for
  manual control (trace-mode debugging, inspection).
- `M.get_grammar()` — exposes the cached `ParserGrammar` for inspection.
- Grammar caching: `toml.grammar` is read from disk and parsed exactly once
  per process; all subsequent calls reuse the cached grammar.
- Path navigation via `debug.getinfo` + `dirname`/`up` helpers, consistent
  with the `toml_lexer` and `json_parser` packages.
- Full test suite (`tests/test_toml_parser.lua`) covering:
  - Module surface (VERSION, parse, create_parser, get_grammar)
  - Root node has rule_name "document"
  - Key-value pairs: strings, integers, floats, booleans (true/false)
  - Multiple key-value pairs in one document
  - Bare keys, quoted keys, dotted keys (a.b.c)
  - Table headers [section] and dotted table headers [a.b]
  - Multiple table sections
  - Array-of-tables headers [[products]]
  - Inline arrays (empty, integers, strings, nested)
  - Inline tables (empty, single pair, multi-pair)
  - Multi-section realistic TOML config document
  - create_parser round-trip
  - Error handling: missing equals, unterminated array, unterminated header
- `BUILD` and `BUILD_windows` with transitive dependency installation in
  leaf-to-root order: state_machine → directed_graph → grammar_tools →
  lexer → toml_lexer → parser → toml_parser.
- `required_capabilities.json` declaring `filesystem:read`.
- `README.md` with architecture description, grammar listing, TOML-specific
  notes about newline significance, and usage examples.
