# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- Initial release of the TOML parser crate.
- `create_toml_parser()` factory function returning a `GrammarParser` configured for TOML.
- `parse_toml()` convenience function returning a `GrammarASTNode` directly.
- Loads the `toml.grammar` file at runtime from the shared `grammars/` directory.
- Depends on `coding-adventures-toml-lexer` for tokenization.
- Supports all TOML grammar rules: document, expression, keyval, key, simple_key, table_header, array_table_header, value, array, array_values, inline_table.
- 21 unit tests covering: simple key-value pairs, integer/boolean/float/datetime values, table headers, array-of-tables headers, dotted keys, inline tables, arrays (single-line and multi-line), multiple key-value pairs, empty documents, comment-only documents, nested table headers, literal string values, quoted table headers, and a full multi-section document integration test.
