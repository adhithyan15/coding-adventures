# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-20

### Added

- Initial release of the JSON lexer crate.
- `create_json_lexer()` factory function returning a `GrammarLexer` configured for JSON.
- `tokenize_json()` convenience function returning `Vec<Token>` directly.
- Loads the `json.tokens` grammar file at runtime from the shared `grammars/` directory.
- Supports all JSON token types: STRING, NUMBER, TRUE, FALSE, NULL, LBRACE, RBRACE, LBRACKET, RBRACKET, COLON, COMMA.
- Whitespace (spaces, tabs, newlines, carriage returns) is silently skipped.
- 17 unit tests covering numbers (integer, negative, decimal, exponent), strings (basic, escapes, empty), literals (true/false/null), structural tokens, objects, arrays, nested structures, empty containers, and the factory function.
