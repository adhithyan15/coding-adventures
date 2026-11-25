## 0.1.0

- Initial Dart lane of the flow-matic-lexer identity: `tokenizeFlowMatic` /
  `tryTokenizeFlowMatic` / `createFlowMaticLexer` over the compiled
  `flow_matic.tokens` grammar (`lib/src/_grammar.dart`, generated with
  the in-lane emitter) — hyphenated names, case-folded keywords,
  numbers, punctuation, and skipped whitespace, with verbatim
  grammar-name types and a final `EOF`. The throwing one-shot
  propagates the engine's structured `LexerError`; the nullable twin
  returns `null` instead.
- Tests mirror the reference suite (all 15 tests) and add the
  untested-corner matrix, 9 seeded property tests over the in-lane
  `pbt` package, and a 2,000-iteration hostile fuzz campaign over
  the in-lane `fuzz` package; 100% line and function coverage.
