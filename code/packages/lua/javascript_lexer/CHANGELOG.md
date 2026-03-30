# Changelog — coding-adventures-javascript-lexer (Lua)

All notable changes to this package are documented here.

## [0.1.0] — 2026-03-29

### Added

- Initial implementation of `coding_adventures.javascript_lexer`.
- `tokenize(source)` — tokenizes a JavaScript string using the shared
  `javascript.tokens` grammar and the grammar-driven `GrammarLexer` from
  `coding-adventures-lexer`.
- `get_grammar()` — returns the cached `TokenGrammar` for direct use.
- Grammar is read from `code/grammars/javascript.tokens` once and cached.
- Path navigation uses `debug.getinfo` to locate the grammar file relative
  to the installed module, avoiding hardcoded absolute paths.
- Full token set: NAME, NUMBER, STRING, keyword tokens (LET, CONST, VAR,
  IF, ELSE, WHILE, FOR, DO, FUNCTION, RETURN, CLASS, IMPORT, EXPORT, FROM,
  AS, NEW, THIS, TYPEOF, INSTANCEOF, TRUE, FALSE, NULL, UNDEFINED),
  operator tokens (STRICT_EQUALS, STRICT_NOT_EQUALS, EQUALS_EQUALS,
  NOT_EQUALS, LESS_EQUALS, GREATER_EQUALS, ARROW, EQUALS, PLUS, MINUS,
  STAR, SLASH, LESS_THAN, GREATER_THAN, BANG), and delimiter tokens
  (LPAREN, RPAREN, LBRACE, RBRACE, LBRACKET, RBRACKET, COMMA, COLON,
  SEMICOLON, DOT).
- Comprehensive busted test suite covering keywords, identifiers, numbers,
  strings, operators, punctuation, arrow functions, whitespace handling,
  position tracking, and error cases.
- `required_capabilities.json` declaring `filesystem:read` (reads grammar
  file at startup).
- `BUILD` and `BUILD_windows` scripts with transitive dependency
  installation in leaf-to-root order.
