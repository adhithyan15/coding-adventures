## 0.1.0

- Initial Dart lane of the twig-lexer identity: `tokenizeTwig` /
  `createTwigLexer` over the compiled `twig.tokens` grammar
  (`lib/src/_grammar.dart`, generated with the in-lane emitter) plus
  the `twigTokenGrammarSpec` accessor — integers, booleans, names,
  keywords, punctuation, comments, and skipped whitespace, with
  verbatim grammar-name types and a final `EOF`. Lex failures throw
  the engine's structured `LexerError`, matching the reference's
  never-panics `Result` design.
- Tests mirror the reference suite (all 22 tests) and add the
  untested-corner matrix, 9 seeded property tests over the in-lane
  `pbt` package, and a 2,000-iteration hostile fuzz campaign over
  the in-lane `fuzz` package; 100% line and function coverage.
