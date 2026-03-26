# Changelog

All notable changes to the `coding-adventures-excel-parser` crate will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- `create_excel_parser(source)` — factory function that loads `excel.grammar` and returns a configured `GrammarParser`.
- `parse_excel(source)` — convenience function that parses JavaScript source and returns a `GrammarASTNode`.
- Loads grammar from `excel.grammar` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite covering variable declarations, expressions, function declarations, if/else, while loops, for loops, multiple statements, empty programs, function calls, and the factory function.
