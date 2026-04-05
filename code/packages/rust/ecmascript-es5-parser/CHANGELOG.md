# Changelog

All notable changes to the `coding-adventures-ecmascript-es5-parser` crate will be documented in this file.

## [0.1.0] - 2026-04-05

### Added
- `create_es5_parser(source)` — factory function that loads `es5.grammar` and returns a configured `GrammarParser`.
- `parse_es5(source)` — convenience function that parses ES5 source and returns a `GrammarASTNode`.
- Loads grammar from `es5.grammar` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite covering ES5-specific features: debugger statement, debugger in function bodies, plus inherited ES3 features (try/catch, throw, strict equality) and ES1 features (variable declarations, function declarations, multiple statements, empty programs).
