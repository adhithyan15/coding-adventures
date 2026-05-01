# Changelog — coding-adventures-algol-lexer

All notable changes to this package are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Accepted uppercase and mixed-case ALGOL keywords in the wrapper without
  lowercasing identifiers or string literal contents.
- Accepted `COMMENT`/mixed-case `comment` as comment starters while preserving
  identifiers such as `commentary` through keyword-boundary matching.
- Accepted `<>` as an alternate ASCII spelling for ALGOL not-equal.
- Accepted double-quoted string literals in addition to single-quoted strings.

## [0.1.0] — 2026-04-06

### Added

- `tokenize_algol(source: str) -> list[Token]` — main entry point; tokenizes
  ALGOL 60 source text into a flat list of tokens.
- `create_algol_lexer(source: str) -> GrammarLexer` — factory function for
  callers who want direct control over the lexer object.
- Grammar-driven implementation using `algol.tokens` and the `GrammarLexer`
  engine from `coding-adventures-lexer`.
- Full support for all ALGOL 60 token types:
  - Value tokens: `INTEGER_LIT`, `REAL_LIT` (with exponent), `STRING_LIT`,
    `IDENT`
  - Multi-character operators: `ASSIGN` (`:=`), `POWER` (`**`), `LEQ` (`<=`),
    `GEQ` (`>=`), `NEQ` (`!=`, `<>`)
  - Single-character operators: `PLUS`, `MINUS`, `STAR`, `SLASH`, `CARET`,
    `EQ`, `LT`, `GT`
  - Delimiters: `LPAREN`, `RPAREN`, `LBRACKET`, `RBRACKET`, `SEMICOLON`,
    `COMMA`, `COLON`
  - Keywords (case-insensitive): `BEGIN`, `END`, `IF`, `THEN`, `ELSE`, `FOR`,
    `DO`, `STEP`, `UNTIL`, `WHILE`, `GOTO`, `SWITCH`, `PROCEDURE`, `OWN`,
    `ARRAY`, `LABEL`, `VALUE`, `INTEGER`, `REAL`, `BOOLEAN`, `STRING`,
    `TRUE`, `FALSE`, `NOT`, `AND`, `OR`, `IMPL`, `EQV`, `DIV`, `MOD`
- Comment skipping: `comment text;` consumed silently (no token emitted).
- Whitespace skipping: spaces, tabs, carriage returns, newlines all ignored.
- Comprehensive pytest test suite with >90% coverage covering:
  - All keyword types (case-insensitive matching)
  - Boolean operator keywords
  - All operator tokens (emphasizing multi-char before single-char ordering)
  - Assignment `:=` vs equality `=` distinction
  - Integer literals, real literals (all exponent forms), string literals
  - Comment skipping behavior
  - Keyword boundary enforcement (`beginning` → IDENT not BEGIN)
  - Identifier tokenization rules
  - Full expression and program tokenization
  - Whitespace insignificance
  - EOF token presence
