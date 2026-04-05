# Changelog

All notable changes to the `coding-adventures-ecmascript-es3-parser` crate will be documented in this file.

## [0.1.0] - 2026-04-05

### Added
- `create_es3_parser(source)` — factory function that loads `es3.grammar` and returns a configured `GrammarParser`.
- `parse_es3(source)` — convenience function that parses ES3 source and returns a `GrammarASTNode`.
- Loads grammar from `es3.grammar` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite covering ES3-specific features: try/catch, try/finally, throw statements, strict equality expressions, plus inherited ES1 features (variable declarations, function declarations, if statements, multiple statements, empty programs).
