# Changelog

## 0.1.0 — 2026-03-20

### Added
- `GrammarLexer.tokenize/2` — grammar-driven tokenization engine
- `Token` struct with type, value, line, column fields
- Standard (non-indentation) tokenization mode
- Skip pattern support (grammar-defined whitespace/comment handling)
- Keyword detection and reclassification (NAME → KEYWORD)
- Reserved keyword checking (raises error on reserved identifiers)
- Type alias resolution (e.g., STRING_DQ → STRING)
- String escape processing: `\n`, `\t`, `\r`, `\b`, `\f`, `\\`, `\"`, `\/`, `\uXXXX`
- Position tracking (line and column numbers)
- First-match-wins priority ordering from `.tokens` file
- JSON grammar integration tests
