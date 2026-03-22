# Changelog

## 0.1.0 — 2026-03-21

### Added
- `TomlLexer.tokenize/1` — tokenize TOML source code
- `TomlLexer.create_lexer/0` — parse toml.tokens grammar
- Grammar caching via `persistent_term` for repeated use
- Support for all TOML v1.0.0 token types: strings (4 types), integers (4 bases), floats (decimal, scientific, special), booleans, date/time (4 types), bare keys, delimiters
- `escapes: none` mode — escape sequences preserved as raw text for semantic layer processing
- 43 tests covering grammar loading, key-value pairs, all string types, integers, floats, booleans, date/time, delimiters, table headers, comments, whitespace, compound documents, position tracking, and errors
