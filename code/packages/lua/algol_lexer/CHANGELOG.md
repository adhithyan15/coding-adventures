# Changelog — coding-adventures-algol-lexer (Lua)

All notable changes to this package are documented here.

## [0.1.0] — 2026-04-06

### Added

- Initial implementation of `coding_adventures.algol_lexer`.
- `tokenize(source)` — tokenizes an ALGOL 60 string using the shared `algol.tokens`
  grammar and the grammar-driven `GrammarLexer` from `coding-adventures-lexer`.
- `get_grammar()` — returns the cached `TokenGrammar` for direct use.
- Grammar is read from `code/grammars/algol.tokens` once and cached.
- Path navigation uses `debug.getinfo` to locate the grammar file relative to
  the installed module, avoiding hardcoded absolute paths.
- Supports all ALGOL 60 token types: keywords (BEGIN, END, IF, THEN, ELSE, FOR,
  DO, STEP, UNTIL, WHILE, GOTO, SWITCH, PROCEDURE, OWN, ARRAY, LABEL, VALUE,
  INTEGER, REAL, BOOLEAN, STRING, TRUE, FALSE, NOT, AND, OR, IMPL, EQV, DIV,
  MOD), identifiers (IDENT), integer and real literals (INTEGER_LIT, REAL_LIT),
  string literals (STRING_LIT), all operators (ASSIGN, POWER, LEQ, GEQ, NEQ,
  PLUS, MINUS, STAR, SLASH, CARET, EQ, LT, GT), and delimiters (LPAREN, RPAREN,
  LBRACKET, RBRACKET, SEMICOLON, COMMA, COLON).
- Whitespace and `comment ... ;` blocks are consumed silently via skip patterns.
- Keywords are case-insensitive; `BEGIN`, `Begin`, and `begin` all produce BEGIN.
- Keyword boundary enforcement: `beginning` is IDENT, not BEGIN + suffix.
- Multi-character operators matched before single-character (:= before :, ** before *, etc.).
- Comprehensive busted test suite covering all token types, comment skipping,
  keyword boundary enforcement, operator disambiguation, position tracking, and
  error cases.
- `required_capabilities.json` declaring `filesystem:read` (reads grammar file
  at startup).
- `BUILD` and `BUILD_windows` scripts with transitive dependency installation
  in leaf-to-root order.
