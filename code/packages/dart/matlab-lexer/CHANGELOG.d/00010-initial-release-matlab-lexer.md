## 0.1.0

- Initial Dart lane of the matlab-lexer identity: `tokenizeMatlab`
  / `tryTokenizeMatlab` / `createMatlabLexer` over the compiled
  `matlab.tokens` grammar (`lib/src/_grammar.dart`, generated with
  the in-lane emitter) plus the four hooks: block-comment strip and
  quote-protection pre-tokenize (transpose-vs-char-array
  disambiguation with `...` continuation splicing), placeholder
  restore and paren-newline drop post-tokenize. Verbatim
  grammar-name types and a final `EOF`; lex failures throw the
  engine's structured `LexerError`, and the nullable twin returns
  `null` instead.
- Tests mirror the reference suite (all 20 tests), 9 seeded
  property tests over the in-lane `pbt` package, and a
  2,000-iteration hostile fuzz campaign over the in-lane `fuzz`
  package; 100% line and function coverage.
