# Changelog

All notable changes to the `coding-adventures-javascript-lexer` crate will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- `create_javascript_lexer(source)` — factory function that loads `javascript.tokens` and returns a configured `GrammarLexer`.
- `tokenize_javascript(source)` — convenience function that tokenizes JavaScript source and returns `Vec<Token>`.
- Loads grammar from `javascript.tokens` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite covering variable declarations, keywords, arithmetic operators, multi-character operators, strings, numbers, comments, delimiters, whitespace, function expressions, arrow operators, and the factory function.
