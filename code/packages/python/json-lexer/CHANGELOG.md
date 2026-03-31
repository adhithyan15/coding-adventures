# Changelog

All notable changes to the JSON lexer package will be documented in this file.

## [0.1.1] - 2026-03-31

### Fixed

- `TestStringEscapes` now uses a local `escape_processing_grammar()` helper
  (a grammar identical to `json.tokens` but without `escapes: none`) instead
  of the real JSON grammar. The real JSON grammar intentionally leaves escape
  sequences raw for the parser to decode; the escape tests were incorrectly
  expecting the lexer to process them. The tests now correctly target the
  lexer engine's escape processing capability while leaving the JSON grammar
  semantics unchanged.

## [0.1.0] - 2026-03-20

### Added
- Initial release of the JSON lexer thin wrapper.
- `tokenize_json()` function for one-step tokenization of JSON text.
- `create_json_lexer()` factory for creating configured `GrammarLexer` instances.
- Full RFC 8259 token support: STRING, NUMBER, TRUE, FALSE, NULL, and all
  structural delimiters ({, }, [, ], :, ,).
- Whitespace (including newlines) handled via skip patterns — no NEWLINE tokens.
- Validates the grammar-driven infrastructure on the simplest practical grammar.
