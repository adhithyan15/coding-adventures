# Changelog

## [0.1.0] - 2026-03-18

### Added
- `Tokenizer` class -- hand-written lexer (NAME, NUMBER, STRING, KEYWORD, operators, delimiters)
- `GrammarLexer` class -- grammar-driven lexer that reads `.tokens` files via grammar_tools
- `Token` immutable data type with type, value, line, column
- `TokenType` module with 16 token type constants
- `LexerError` exception with line and column information
- Keyword support via configurable keyword list
- Escape sequence handling in string literals (\\n, \\t, \\\\, \\")
