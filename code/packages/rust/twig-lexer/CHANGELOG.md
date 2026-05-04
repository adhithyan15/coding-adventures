# Changelog — twig-lexer

## [0.2.0] — 2026-05-04

### Added (LANG23 PR 23-E — refinement type annotation tokens)

- `COLON` token (`:`): promotes the colon character from a lex error to a
  dedicated punctuation token, enabling `(x : (Int 0 128))` parameter
  annotations and `(define x : (Int 0 128) val)` annotated value bindings.
- `ARROW` token (`->`): exact-literal token that takes priority over the `NAME`
  pattern in the lexer, preventing the return-type annotation marker from being
  consumed as a bare-NAME parameter in `{ typed_param }` repetitions.  Without
  this token, `(define (f (x : int) -> (Int 0 256)) body)` would fail with
  "Expected COLON, got '0'".
- Tests for `COLON` and `ARROW` token lexing, including boundary checks that
  `-` alone still lexes as `NAME` (longest-match: `->` is ARROW, `-` is NAME).
- Token-table documentation updated to include `COLON` and `ARROW` entries.

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
