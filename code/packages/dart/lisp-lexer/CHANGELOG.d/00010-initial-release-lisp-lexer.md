## 0.1.0

- Initial Dart lane of the lisp-lexer identity: hand-written
  tokenizer for atoms, delimiters, strings, and EOF with the
  reference's exact error texts; rune-offset positions matching the
  reference's `char` walk. (A grammar-driven migration follows
  separately once the shared lexer engine lands.)
- Tests mirror the reference suite (35 unit tests), add 6 seeded
  property tests over the in-lane `pbt` package and a 2,000-iteration
  hostile fuzz campaign over the in-lane `fuzz` package; 100%
  line/branch/function coverage.
