# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- `token` module with `TokenType` enum (23 variants), `Token` struct, and `LexerError` type.
- `tokenizer` module — hand-written character-by-character Python lexer with:
  - Configurable keyword set for keyword promotion (NAME -> KEYWORD).
  - String literal support with escape sequence processing (\n, \t, \\, \").
  - Lookahead for multi-character operators (= vs ==).
  - Single-character token lookup table for operators and delimiters.
  - Line and column position tracking for error messages.
  - Comprehensive error reporting for unexpected characters and unterminated strings.
- `grammar_lexer` module — grammar-driven universal lexer with:
  - Accepts a `TokenGrammar` from the `grammar-tools` crate.
  - Compiles grammar patterns into anchored regexes at construction time.
  - First-match-wins semantics matching the grammar's definition order.
  - Keyword promotion from NAME to KEYWORD using the grammar's keyword list.
  - String escape processing matching the hand-written lexer's behavior.
  - Consistency tests verifying identical output between both lexer implementations.
