# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] - 2026-04-04

### Added

- Token flag constants: `TOKEN_PRECEDED_BY_NEWLINE` (1) and `TOKEN_CONTEXT_KEYWORD` (2)
- `flags` field on Token (optional bitmask, defaults to 0)
- LexerContext extensions:
  - `previous_token()` — lookbehind for context-sensitive decisions
  - `bracket_depth(kind)` — per-type (`paren`, `bracket`, `brace`) or total nesting depth
  - `preceded_by_newline()` — newline detection for ASI-like languages
- GrammarLexer extensions:
  - `_last_emitted_token` tracking for lookbehind
  - Per-type `_bracket_depths` tracking (paren/bracket/brace)
  - `_context_keyword_set` for context-sensitive keyword flagging
  - `bracket_depth(kind)` public method
  - `_update_bracket_depth(value)` internal helper
  - Automatic `TOKEN_CONTEXT_KEYWORD` flag on NAME tokens matching context keywords

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
