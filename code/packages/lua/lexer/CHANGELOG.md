# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- Token type constants (Name, Number, String, Keyword, Plus, Minus, Star, Slash, Equals, EqualsEquals, LParen, RParen, Comma, Colon, Semicolon, LBrace, RBrace, LBracket, RBracket, Dot, Bang, Newline, EOF)
- Token class with type, value, line, column, and type_name fields
- classify_char function mapping characters to DFA character classes
- Tokenizer DFA construction using state_machine.DFA for dispatch logic
- Hand-written Lexer with support for:
  - Identifiers (names) with letters, digits, and underscores
  - Integer number literals
  - Double-quoted string literals with escape sequences (\\n, \\t, \\\\, \\")
  - Single-character operators and delimiters
  - Equals (=) vs double-equals (==) via one-character lookahead
  - Configurable keyword promotion
  - Whitespace skipping and newline tokens
  - Position tracking (line and column)
- Grammar-driven GrammarLexer with support for:
  - Lua pattern-based token definitions (regex and literal)
  - Skip patterns (whitespace, comments)
  - Type aliases (e.g., STRING_DQ -> STRING)
  - Reserved keywords (lex-time errors)
  - Keyword promotion (NAME -> KEYWORD)
  - String escape processing with escape_mode control
  - Single and triple-quoted string support
  - Indentation mode (INDENT/DEDENT/NEWLINE tokens)
  - Bracket-aware implicit line continuation
  - Pattern groups with stackable group transitions
  - On-token callback with LexerContext for:
    - Group push/pop
    - Token emission and suppression
    - Peek/peek_str lookahead
    - Skip toggle
- process_escapes utility function
- 96 busted tests covering all features
- Ported from Go implementation at code/packages/go/lexer/
