## 0.1.0

- Initial Dart lane of the apl-lexer identity: `tokenizeApl` /
  `createAplLexer` over the compiled `apl.tokens` grammar
  (`lib/src/_grammar.dart`, generated with the in-lane emitter) —
  primitive glyphs, comparisons, operators, high-minus numbers,
  names, significant newlines, and skipped whitespace/comments, with
  verbatim grammar-name types and a final `EOF`. Lex failures throw
  the engine's structured `LexerError`.
- Tests mirror the reference suite (all 10 integration tests) and add
  the untested-corner matrix, 9 seeded property tests over the
  in-lane `pbt` package, and a 2,000-iteration hostile fuzz campaign
  over the in-lane `fuzz` package; 100% line and function coverage.
  All unit inputs were additionally verified token-for-token against
  the Rust implementation via a differential probe.
