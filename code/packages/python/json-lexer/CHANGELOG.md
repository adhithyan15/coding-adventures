# Changelog

All notable changes to the JSON lexer package will be documented in this file.

## [0.1.0] - 2026-03-20

### Added
- Initial release of the JSON lexer thin wrapper.
- `tokenize_json()` function for one-step tokenization of JSON text.
- `create_json_lexer()` factory for creating configured `GrammarLexer` instances.
- Full RFC 8259 token support: STRING, NUMBER, TRUE, FALSE, NULL, and all
  structural delimiters ({, }, [, ], :, ,).
- Whitespace (including newlines) handled via skip patterns — no NEWLINE tokens.
- Validates the grammar-driven infrastructure on the simplest practical grammar.
