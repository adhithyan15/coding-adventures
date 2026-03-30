# Changelog — coding-adventures-lattice-lexer (Lua)

All notable changes to this package are documented here.

## [0.1.0] — 2026-03-29

### Added

- Initial implementation of `coding_adventures.lattice_lexer`.
- `tokenize(source)` — tokenizes a Lattice string using the shared
  `lattice.tokens` grammar and the grammar-driven `GrammarLexer` from
  `coding-adventures-lexer`.
- `get_grammar()` — returns the cached `TokenGrammar` for direct use.
- Grammar is read from `code/grammars/lattice.tokens` once and cached.
- Path navigation uses `debug.getinfo` to locate the grammar file relative
  to the installed module, avoiding hardcoded absolute paths.
- Full token set covering all CSS tokens plus Lattice extensions:
  Lattice-specific: VARIABLE, PLACEHOLDER, EQUALS_EQUALS, NOT_EQUALS,
  GREATER_EQUALS, LESS_EQUALS, BANG_DEFAULT, BANG_GLOBAL;
  Numeric: DIMENSION, PERCENTAGE, NUMBER;
  CSS identifiers and values: STRING, HASH, AT_KEYWORD, URL_TOKEN,
  FUNCTION, CUSTOM_PROPERTY, IDENT, UNICODE_RANGE;
  CSS attribute operators: COLON_COLON, TILDE_EQUALS, PIPE_EQUALS,
  CARET_EQUALS, DOLLAR_EQUALS, STAR_EQUALS;
  CSS delimiters: LBRACE, RBRACE, LPAREN, RPAREN, LBRACKET, RBRACKET,
  SEMICOLON, COLON, COMMA, DOT, AMPERSAND;
  Other operators: PLUS, GREATER, LESS, TILDE, STAR, PIPE, BANG,
  SLASH, EQUALS, MINUS; CDO/CDC legacy tokens.
- `escapes: none` mode respected — string values include raw escape
  sequences (CSS escape decoding is a semantic post-parse concern).
- Comprehensive busted test suite covering Lattice variables, placeholder
  selectors, numeric priority ordering (DIMENSION > PERCENTAGE > NUMBER),
  hash tokens, at-keywords, URL tokens, function tokens, identifiers,
  custom properties, multi-character operators, bang tokens, delimiter
  tokens, string literals, comment handling, composite expressions,
  whitespace handling, position tracking, and error cases.
- `required_capabilities.json` declaring `filesystem:read` (reads grammar
  file at startup).
- `BUILD` and `BUILD_windows` scripts with transitive dependency
  installation in leaf-to-root order.
