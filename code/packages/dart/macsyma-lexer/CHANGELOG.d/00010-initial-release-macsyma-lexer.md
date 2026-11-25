## 0.1.0

- Initial Dart lane of the macsyma-lexer identity: `tokenizeMacsyma` /
  `createMacsymaLexer` over the compiled `macsyma.tokens` grammar
  (`lib/src/_grammar.dart`, generated with the in-lane emitter) —
  names, numbers, percent constants, keywords, operators, brackets,
  and statement terminators, with verbatim grammar-name types and a
  final `EOF`. Lex failures throw the engine's structured
  `LexerError`.
- Tests mirror the reference suite (all 5 tests) and add the
  untested-corner matrix, 9 seeded property tests over the in-lane
  `pbt` package, and a 2,000-iteration hostile fuzz campaign over
  the in-lane `fuzz` package; 100% line and function coverage.
