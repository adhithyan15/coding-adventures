# Changelog

All notable changes to the `coding-adventures-ruby-parser` crate will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- `create_ruby_parser(source)` — factory function that loads `ruby.grammar` and returns a configured `GrammarParser`.
- `parse_ruby(source)` — convenience function that parses Ruby source and returns a `GrammarASTNode`.
- Loads grammar from `ruby.grammar` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite covering assignments, expressions, method definitions, if/else, while loops, multiple statements, empty programs, class definitions, method calls, and the factory function.
