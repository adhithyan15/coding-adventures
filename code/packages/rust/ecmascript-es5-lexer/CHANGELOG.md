# Changelog

All notable changes to the `coding-adventures-ecmascript-es5-lexer` crate will be documented in this file.

## [0.1.0] - 2026-04-05

### Added
- `create_es5_lexer(source)` — factory function that loads `es5.tokens` and returns a configured `GrammarLexer`.
- `tokenize_es5(source)` — convenience function that tokenizes ES5 source and returns `Vec<Token>`.
- Loads grammar from `es5.tokens` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite verifying ES5-specific behavior: `debugger` as a keyword, strict equality inherited from ES3, all ES3 keywords still present, `let`/`const` not as keywords, comments skipped, and the factory function.
