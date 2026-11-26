## 0.1.0

- Initial Dart lane of the oct-lexer identity: `tokenizeOct` /
  `createOctLexer` over the compiled `oct.tokens` grammar
  (`lib/src/_grammar.dart`, generated with the in-lane emitter),
  with the reference's keyword post-pass (each `KEYWORD` token's
  type becomes its literal value). Verbatim grammar-name types and
  a final `EOF`; lex failures throw the engine's structured
  `LexerError`.
- Tests mirror the reference suite (all 5 tests) and add the
  untested-corner matrix, 9 seeded property tests over the in-lane
  `pbt` package, and a 2,000-iteration hostile fuzz campaign over
  the in-lane `fuzz` package; 100% line and function coverage.
