# Changelog

All notable changes to the `coding-adventures-css-lexer` crate will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- `create_css_lexer(source)` — factory function that loads `css.tokens` and returns a configured `GrammarLexer`.
- `tokenize_css(source)` — convenience function that tokenizes CSS source and returns `Vec<Token>`.
- Loads grammar from `css.tokens` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite covering simple rules, dimensions, hash tokens, strings, at-keywords, whitespace/comment skipping, delimiters, multiple selectors, and the factory function.
