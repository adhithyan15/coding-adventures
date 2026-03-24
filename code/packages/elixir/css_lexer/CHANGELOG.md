# Changelog

## 0.1.0 — 2026-03-24

### Added
- `CssLexer.tokenize/1` — tokenize CSS source code from `css.tokens`
- `CssLexer.create_lexer/0` — parse and return the shared CSS token grammar
- Grammar caching via `persistent_term` for repeated calls
- Tests covering CSS compound tokens, functions, selectors, operators, positions, and errors
