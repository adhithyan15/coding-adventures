# Changelog

## 0.1.0 — 2026-03-24

### Added
- `RubyLexer.tokenize/1` — tokenize Ruby source code from `ruby.tokens`
- `RubyLexer.create_lexer/0` — parse and return the shared Ruby token grammar
- Grammar caching via `persistent_term` for repeated calls
- Tests covering keywords, Ruby-specific operators, strings, positions, and errors
