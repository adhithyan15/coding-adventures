# Changelog

All notable changes to the `coding-adventures-ecmascript-es1-lexer` crate will be documented in this file.

## [0.1.0] - 2026-04-05

### Added
- `create_es1_lexer(source)` — factory function that loads `es1.tokens` and returns a configured `GrammarLexer`.
- `tokenize_es1(source)` — convenience function that tokenizes ES1 source and returns `Vec<Token>`.
- Loads grammar from `es1.tokens` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite verifying ES1-specific behavior: no `===`/`!==` operators, `try`/`catch`/`instanceof` are identifiers (not keywords), abstract equality (`==`/`!=`), variable declarations, keywords, operators, strings, numbers, delimiters, comments, and the factory function.
