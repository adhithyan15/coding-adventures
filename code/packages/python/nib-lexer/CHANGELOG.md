# Changelog

All notable changes to `coding-adventures-nib-lexer` will be documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-04-12

### Added

- `tokenize_nib(source)` — tokenize Nib source text into a flat list of
  `Token` objects. The list always ends with an EOF token.
- `create_nib_lexer(source)` — factory function that reads `nib.tokens`,
  parses it with `grammar_tools.parse_token_grammar()`, and returns a
  `GrammarLexer` configured for Nib. Use when you need direct control over
  the lexer lifecycle.
- Full token coverage for the Nib language as defined in `nib.tokens`:
  - Multi-character operators: `WRAP_ADD` (`+%`), `SAT_ADD` (`+?`), `RANGE`
    (`..`), `ARROW` (`->`), `EQ_EQ` (`==`), `NEQ` (`!=`), `LEQ` (`<=`),
    `GEQ` (`>=`), `LAND` (`&&`), `LOR` (`||`)
  - Single-character arithmetic: `PLUS`, `MINUS`, `STAR`, `SLASH`
  - Single-character bitwise: `AMP`, `PIPE`, `CARET`, `TILDE`
  - Comparison/logical: `BANG`, `LT`, `GT`, `EQ`
  - Delimiters: `LBRACE`, `RBRACE`, `LPAREN`, `RPAREN`, `COLON`,
    `SEMICOLON`, `COMMA`
  - Literals: `HEX_LIT` (`0x[0-9A-Fa-f]+`), `INT_LIT` (`[0-9]+`)
  - Identifiers: `NAME` (reclassified to keyword on exact match)
  - Keywords: `fn`, `let`, `static`, `const`, `return`, `for`, `in`, `if`,
    `else`, `true`, `false`
  - Skipped: `WHITESPACE`, `LINE_COMMENT` (`//` to end of line)
- 80+ test cases covering every token type, multi-char disambiguation,
  keyword boundary enforcement, type-as-NAME behavior, comment skipping,
  whitespace handling, EOF, and complete statement sequences.
- Literate docstrings explaining the Intel 4004 hardware constraints,
  why `+%` and `+?` are separate tokens, why `HEX_LIT` must precede
  `INT_LIT`, and the grammar-driven first-match-wins approach.
- `README.md` with token table, architecture diagram, usage examples, and
  Intel 4004 hardware context.
