# Changelog

All notable changes to the `coding-adventures-starlark-parser` crate will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- `create_starlark_parser(source)` — factory function that tokenizes source with starlark-lexer, loads `starlark.grammar`, and returns a configured `GrammarParser`.
- `parse_starlark(source)` — convenience function that parses Starlark source and returns a `GrammarASTNode`.
- Loads grammar from `starlark.grammar` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Comprehensive test suite covering simple assignments, expressions, function definitions, if/else, for loops, BUILD file patterns, multiple statements, and the factory function.
