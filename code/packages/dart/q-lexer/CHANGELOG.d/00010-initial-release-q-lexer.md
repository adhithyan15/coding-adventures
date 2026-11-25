## 0.1.0

- Initial Dart lane of the q-lexer identity: `tokenizeQ` /
  `tryTokenizeQ` / `createQLexer` over the compiled `q.tokens`
  grammar (`lib/src/_grammar.dart`, generated with the in-lane
  emitter) plus the `stripSlashComments` pre-tokenize hook and the
  `foldNegativeNumberLiterals` post-tokenize hook. Verbatim
  grammar-name types and a final `EOF`; lex failures throw the
  engine's structured `LexerError`, and the nullable twin returns
  `null` instead.
- Tests mirror the reference suite (all 5 in-module tests and all 47
  integration tests) and add the untested-corner matrix, 9 seeded
  property tests over the in-lane `pbt` package, and a 2,000-iteration
  hostile fuzz campaign over the in-lane `fuzz` package; 100% line
  and function coverage.
