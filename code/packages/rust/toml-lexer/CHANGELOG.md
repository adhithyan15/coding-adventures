# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- Initial release of the TOML lexer crate.
- `create_toml_lexer()` factory function returning a `GrammarLexer` configured for TOML.
- `tokenize_toml()` convenience function returning `Vec<Token>` directly.
- Loads the `toml.tokens` grammar file at runtime from the shared `grammars/` directory.
- Supports all TOML token types: BARE_KEY, BASIC_STRING, LITERAL_STRING, ML_BASIC_STRING, ML_LITERAL_STRING, INTEGER (decimal, hex, octal, binary), FLOAT (decimal, scientific, inf, nan), TRUE, FALSE, OFFSET_DATETIME, LOCAL_DATETIME, LOCAL_DATE, LOCAL_TIME, EQUALS, DOT, COMMA, LBRACKET, RBRACKET, LBRACE, RBRACE, NEWLINE.
- Uses `escapes: none` mode — quotes are stripped but escape sequences are left as raw text for type-specific processing by the semantic layer.
- Comments (# to end of line) are silently skipped.
- 31 unit tests covering: bare keys, basic strings, literal strings, integers (decimal, underscores, hex, octal, binary), floats (decimal, scientific, inf, nan), booleans, all 4 date/time types, structural tokens, key-value pairs, table headers, array-of-tables headers, dotted keys, inline tables, arrays, newlines, comments, multi-line strings, negative integers, quoted keys, and a full document integration test.
