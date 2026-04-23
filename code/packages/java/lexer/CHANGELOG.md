# Changelog

## 0.1.0 — 2026-04-04

### Added
- `Token` class with type, value, line, column, typeName, and flags
- `TokenType` enum for standard token types
- `GrammarLexer` — grammar-driven tokenizer using TokenGrammar
- Keyword promotion, type aliases, reserved keyword detection
- Context-sensitive keyword flags
- Preceded-by-newline tracking
- Error recovery pattern support
- Full test suite
