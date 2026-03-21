# Changelog

All notable changes to the `coding-adventures-typescript-lexer` crate will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- `create_typescript_lexer(source)` — factory function that loads `typescript.tokens` and returns a configured `GrammarLexer`.
- `tokenize_typescript(source)` — convenience function that tokenizes TypeScript source and returns `Vec<Token>`.
- Loads grammar from `typescript.tokens` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite covering typed declarations, keywords (including TypeScript-specific), arithmetic operators, multi-character operators, strings, numbers, comments, delimiters, whitespace, arrow operators, angle brackets, and the factory function.
