## 0.1.0

- Initial Dart lane of the j-lexer identity: `tokenizeJ` /
  `createJLexer` over the compiled `j.tokens` grammar
  (`lib/src/_grammar.dart`, generated with the in-lane emitter) —
  verbs, digraphs, adverbs, numbers, names, comments, and significant
  newlines, with verbatim grammar-name types and a final `EOF`. Lex
  failures throw the engine's structured `LexerError`.
- Tests mirror the reference suite (all 15 tests) and add the
  untested-corner matrix, 9 seeded property tests over the in-lane
  `pbt` package, and a 2,000-iteration hostile fuzz campaign over
  the in-lane `fuzz` package; 100% line and function coverage. All
  unit inputs were additionally verified token-for-token against the
  Rust implementation via a differential probe.
