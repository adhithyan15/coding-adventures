## 0.1.0

- Initial Dart lane of the jsdoc-lexer identity: `tokenizeJsdoc` /
  `createJsdocLexer` over the compiled `jsdoc.tokens` grammar
  (`lib/src/_grammar.dart`, generated with the in-lane emitter).
  Pure engine call, no post-pass — and no nullable twin, matching
  the reference (callers use factory + `tokenize()` for explicit
  error handling). Verbatim grammar-name types and a final `EOF`;
  lex failures throw the engine's structured `LexerError`.
- Tests mirror the reference suite (all 9 tests), 9 seeded property
  tests over the in-lane `pbt` package, and a 2,000-iteration
  hostile fuzz campaign over the in-lane `fuzz` package; 100% line
  and function coverage.
