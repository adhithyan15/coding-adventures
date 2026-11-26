## 0.1.0

- Initial Dart lane of the derive-lexer identity: `tokenizeDerive`
  / `tryTokenizeDerive` / `createDeriveLexer` over the compiled
  `derive.tokens` grammar (`lib/src/_grammar.dart`, generated with
  the in-lane emitter) plus the `dropBracketedNewlines`
  post-tokenize hook (combined `(`/`[` depth, saturating on
  unbalanced closers). Verbatim grammar-name types and a final
  `EOF`; lex failures throw the engine's structured `LexerError`,
  and the nullable twin returns `null` instead.
- Tests mirror the reference suite (all 13 tests), 9 seeded
  property tests over the in-lane `pbt` package, and a
  2,000-iteration hostile fuzz campaign over the in-lane `fuzz`
  package; 100% line and function coverage.
