# Changelog

All notable changes to the `coding-adventures-typescript-parser` crate will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- `create_typescript_parser(source)` — factory function that loads `typescript.grammar` and returns a configured `GrammarParser`.
- `parse_typescript(source)` — convenience function that parses TypeScript source and returns a `GrammarASTNode`.
- Loads grammar from `typescript.grammar` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite covering typed declarations, expressions, function declarations, if/else, while loops, for loops, multiple statements, empty programs, interface declarations, and the factory function.
