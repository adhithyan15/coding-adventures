# Changelog

All notable changes to `coding_adventures_json_lexer` will be documented in this file.

## [0.1.0] - 2026-03-20

### Added
- Initial release
- `CodingAdventures::JsonLexer.tokenize(source)` method that tokenizes JSON text
- Loads `json.tokens` grammar file and delegates to `GrammarLexer`
- Supports value tokens: STRING, NUMBER, TRUE, FALSE, NULL
- Supports structural tokens: LBRACE, RBRACE, LBRACKET, RBRACKET, COLON, COMMA
- Supports JSON number formats: integers, negatives, decimals, exponent notation
- Supports JSON string escapes: `\"`, `\\`, `\/`, `\b`, `\f`, `\n`, `\r`, `\t`, `\uXXXX`
- Whitespace (space, tab, CR, LF) silently skipped -- no NEWLINE/INDENT/DEDENT tokens
- Full test suite with SimpleCov coverage >= 80%
