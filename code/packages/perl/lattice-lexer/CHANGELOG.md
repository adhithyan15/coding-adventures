# Changelog — CodingAdventures::LatticeLexer (Perl)

All notable changes to this package are documented here.

## [0.01] — 2026-03-29

### Added

- Initial implementation of `CodingAdventures::LatticeLexer`.
- `tokenize($source)` — tokenizes a Lattice string using the shared
  `lattice.tokens` grammar, compiled to Perl `qr//` regexes.
- Grammar is read from `code/grammars/lattice.tokens` once and cached via
  package-level variables.
- Path navigation uses `dirname(__FILE__)` + 5 dirname() levels to reach
  `code/` from `lib/CodingAdventures/`, avoiding hardcoded absolute paths.
- `escapes: none` mode respected — string values include surrounding quotes
  and raw escape sequences. CSS escape decoding (\26, \A9) is left to the
  semantic layer, not performed at the lexer level.
- Full token set covering all CSS tokens plus Lattice extensions:
  Lattice-specific: VARIABLE, PLACEHOLDER, EQUALS_EQUALS, NOT_EQUALS,
  GREATER_EQUALS, LESS_EQUALS, BANG_DEFAULT, BANG_GLOBAL;
  Numeric (priority DIMENSION > PERCENTAGE > NUMBER): DIMENSION, PERCENTAGE,
  NUMBER; CSS shared: STRING, HASH, AT_KEYWORD, URL_TOKEN, FUNCTION,
  CUSTOM_PROPERTY, IDENT, UNICODE_RANGE, CDO, CDC;
  CSS attribute operators: COLON_COLON, TILDE_EQUALS, PIPE_EQUALS,
  CARET_EQUALS, DOLLAR_EQUALS, STAR_EQUALS; delimiters and operators:
  LBRACE, RBRACE, LPAREN, RPAREN, LBRACKET, RBRACKET, SEMICOLON, COLON,
  COMMA, DOT, AMPERSAND, BANG, SLASH, EQUALS, PLUS, GREATER, LESS, TILDE,
  STAR, PIPE, MINUS.
- Comprehensive Test2::V0 test suite: 00-load.t (module loading), 01-basic.t
  (VARIABLE, PLACEHOLDER, numeric priority ordering, HASH, AT_KEYWORD,
  FUNCTION, IDENT, CUSTOM_PROPERTY, multi-character operators, bang tokens,
  delimiter tokens, string literals, comment handling, composite expressions,
  whitespace handling, position tracking, error cases).
- `required_capabilities.json` declaring `filesystem:read`.
- `BUILD`, `BUILD_windows`, `Makefile.PL`, and `cpanfile` with transitive
  dependency installation in leaf-to-root order.
