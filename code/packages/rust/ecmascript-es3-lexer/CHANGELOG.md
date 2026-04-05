# Changelog

All notable changes to the `coding-adventures-ecmascript-es3-lexer` crate will be documented in this file.

## [0.1.0] - 2026-04-05

### Added
- `create_es3_lexer(source)` — factory function that loads `es3.tokens` and returns a configured `GrammarLexer`.
- `tokenize_es3(source)` — convenience function that tokenizes ES3 source and returns `Vec<Token>`.
- Loads grammar from `es3.tokens` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite verifying ES3-specific behavior: strict equality (`===`/`!==`), `try`/`catch`/`finally`/`throw` as keywords, `instanceof` keyword, `debugger` as reserved (not plain NAME), abstract equality still works, comments skipped, and the factory function.
