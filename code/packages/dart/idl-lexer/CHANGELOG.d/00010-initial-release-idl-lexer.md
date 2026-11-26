## 0.1.0

- Initial Dart lane of the idl-lexer identity: `tokenizeIdl` /
  `tryTokenizeIdl` / `createIdlLexer` over the compiled `idl.tokens`
  grammar (`lib/src/_grammar.dart`, generated with the in-lane
  emitter) — keywords with case-insensitive uppercase folding,
  case-preserving names, strings, numbers, comments, continuations,
  statement separators, hashes, and operators, with verbatim
  grammar-name types and a final `EOF`. The throwing one-shot
  propagates the engine's structured `LexerError`; the nullable twin
  returns `null` instead.
- Tests mirror the reference suite (all 5 in-module tests and all 53
  integration tests) and add the untested-corner matrix, 9 seeded
  property tests over the in-lane `pbt` package, and a 2,000-iteration
  hostile fuzz campaign over the in-lane `fuzz` package; 100% line
  and function coverage.
