## 0.1.0

- Initial Dart lane of the prolog-lexer identity: `tokenizeIsoProlog`
  / `createIsoPrologLexer` over the compiled `iso.tokens` grammar
  (`lib/src/_grammar.dart`, generated with the in-lane emitter) —
  atoms, variables, integers, floats, strings, arrows, operators,
  brackets, and skipped whitespace/comments, with verbatim
  grammar-name types and a final `EOF`. Lex failures throw the
  engine's structured `LexerError`.
- Tests mirror the reference suite (all 17 tests) and add the
  untested-corner matrix, 9 seeded property tests over the in-lane
  `pbt` package, and a 2,000-iteration hostile fuzz campaign over
  the in-lane `fuzz` package; 100% line and function coverage.
