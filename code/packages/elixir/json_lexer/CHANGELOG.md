# Changelog

## 0.1.0 — 2026-03-20

### Added
- `JsonLexer.tokenize/1` — tokenize JSON source code
- `JsonLexer.create_lexer/0` — parse json.tokens grammar
- Grammar caching via `persistent_term` for repeated use
- 16 tests covering primitives, structural tokens, compound structures, whitespace, position tracking, and errors
