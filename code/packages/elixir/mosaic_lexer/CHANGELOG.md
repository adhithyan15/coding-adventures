# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-04

### Added

- `CodingAdventures.MosaicLexer.tokenize/1` — tokenizes Mosaic source using
  the grammar-driven lexer engine driven by `mosaic.tokens`
- `CodingAdventures.MosaicLexer.create_lexer/0` — parses `mosaic.tokens` into
  a `TokenGrammar` struct for introspection or direct reuse
- `:persistent_term` caching so the grammar is only parsed once per VM
- 28 unit tests covering: `create_lexer/0` grammar introspection; keyword
  promotion (KEYWORD tokens); NAME identifiers including hyphenated CSS-like
  names; COLOR_HEX tokens (#rgb, #rrggbb, #rrggbbaa); DIMENSION tokens (16dp,
  50%, 1.5rem); NUMBER tokens; STRING tokens; all structural delimiters (LBRACE,
  RBRACE, LANGLE, RANGLE, COLON, SEMICOLON, COMMA, DOT, EQUALS, AT); slot
  reference tokenization (@name); whitespace and comment skipping; position
  tracking; realistic snippets; error handling
