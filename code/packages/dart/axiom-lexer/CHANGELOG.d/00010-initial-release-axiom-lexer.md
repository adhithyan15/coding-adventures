## 0.1.0

- Initial Dart lane of the axiom-lexer identity: `tokenizeAxiom` /
  `tryTokenizeAxiom` / `createAxiomLexer` over the compiled
  `axiom.tokens` grammar (`lib/src/_grammar.dart`, generated with
  the in-lane emitter) — numbers, strings, names, case-sensitive
  keywords, operators, brackets, comments, and skipped whitespace,
  with verbatim grammar-name types and a final `EOF`. The throwing
  one-shot propagates the engine's structured `LexerError`; the
  nullable twin returns `null` instead.
- Tests mirror the reference suite (all 6 in-module tests and all 56
  integration tests, including the scale tests) and add the
  untested-corner matrix, 9 seeded property tests over the in-lane
  `pbt` package, and a 2,000-iteration hostile fuzz campaign over
  the in-lane `fuzz` package; 100% line and function coverage.
