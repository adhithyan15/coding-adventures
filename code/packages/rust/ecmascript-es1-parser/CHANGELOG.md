# Changelog

All notable changes to the `coding-adventures-ecmascript-es1-parser` crate will be documented in this file.

## [0.1.0] - 2026-04-05

### Added
- `create_es1_parser(source)` — factory function that loads `es1.grammar` and returns a configured `GrammarParser`.
- `parse_es1(source)` — convenience function that parses ES1 source and returns a `GrammarASTNode`.
- Loads grammar from `es1.grammar` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite covering variable declarations, arithmetic expressions, multiple statements, empty programs, function declarations, if statements, while loops, abstract equality parsing, switch statements, and the factory function.
