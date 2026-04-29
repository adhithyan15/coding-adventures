# Changelog — twig-lexer

## [0.1.0] — 2026-04-29

### Added

- Initial Rust implementation of the Twig lexer (TW00).
- Thin wrapper around [`lexer::grammar_lexer::GrammarLexer`](../lexer)
  driven by `code/grammars/twig.tokens` — the canonical Twig token
  grammar shared with the Python implementation.
- `tokenize_twig(source) -> Vec<Token>` — convenience entry that
  reads the grammar from disk, builds the lexer, and tokenises in
  one call.
- `create_twig_lexer(source) -> GrammarLexer` — for callers that
  want the lexer object itself (incremental tokenisation, custom
  error handling).
- Token kinds (per `twig.tokens`): `LPAREN`, `RPAREN`, `QUOTE`,
  `BOOL_TRUE`, `BOOL_FALSE`, `INTEGER`, `KEYWORD`, `NAME`, plus
  the trailing `Eof` sentinel.
- Keyword promotion for `define`, `lambda`, `let`, `if`, `begin`,
  `quote`, `nil` — the parser dispatches on `KEYWORD` tokens.
- 1-indexed `(line, column)` position tracking propagated through
  `Token` (provided by the `GrammarLexer`).
- 16 unit tests covering every token kind, comment/whitespace
  skipping, keyword promotion, negative-integer disambiguation,
  and realistic shapes.
