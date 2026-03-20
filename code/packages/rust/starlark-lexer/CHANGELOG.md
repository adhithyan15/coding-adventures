# Changelog

All notable changes to the `coding-adventures-starlark-lexer` crate will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- `create_starlark_lexer(source)` — factory function that loads `starlark.tokens` and returns a configured `GrammarLexer`.
- `tokenize_starlark(source)` — convenience function that tokenizes Starlark source and returns `Vec<Token>`.
- Loads grammar from `starlark.tokens` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Comprehensive test suite covering simple expressions, keywords, reserved keyword errors, indentation, bracket suppression, operators, strings, comments, float literals, and the factory function.
