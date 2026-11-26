## 0.1.0

- Initial Dart lane of the css-lexer identity: `tokenizeCss` /
  `createCssLexer` over the compiled `css.tokens` grammar
  (`lib/src/_grammar.dart`, generated with the in-lane emitter) —
  dimensions, percentages, numbers, hashes, at-keywords, URLs,
  functions, idents, custom properties, unicode ranges, strings with
  escapes-none values, CDO/CDC, operators, and skipped
  whitespace/comments, with verbatim grammar-name types and a final
  `EOF`. Lex failures throw the engine's structured `LexerError`.
- Tests mirror the reference suite (all 10 tests) and add the
  untested-corner matrix, 8 seeded property tests over the in-lane
  `pbt` package, and a 2,000-iteration hostile fuzz campaign over
  the in-lane `fuzz` package; 100% line and function coverage.
