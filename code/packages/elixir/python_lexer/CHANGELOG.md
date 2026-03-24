# Changelog

## 0.1.0 — 2026-03-24

### Added
- `PythonLexer.tokenize/1` — tokenize Python source code from `python.tokens`
- `PythonLexer.create_lexer/0` — parse and return the shared Python token grammar
- Grammar caching via `persistent_term` for repeated calls
- Tests covering keywords, operators, delimiters, strings, values, positions, and errors
