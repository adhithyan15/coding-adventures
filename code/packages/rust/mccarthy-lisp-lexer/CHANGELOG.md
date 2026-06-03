# Changelog — mccarthy-lisp-lexer

## v0.1.0 — 2026-06-03 — initial release (L1)

McCarthy 1960 Lisp tokenizer.  Recognises all-uppercase atoms,
signed-decimal integers, parens, `'` quote sugar, `.` dotted-pair
separator, and `;` line comments (a Lisp 1.5 convenience).

* `Token` enum with 6 variants + a `Loc { line, column }` triple.
* `tokenize(src) -> Result<Vec<TokenWithLoc>, LexError>`.
* `LexError` carries the offending byte, line, column, and a
  human-readable reason.

Tests pin every token shape and the standard McCarthy example
sources from the 1960 paper (`(CAR '(A B C))`, `(LAMBDA (X) X)`,
etc.).
