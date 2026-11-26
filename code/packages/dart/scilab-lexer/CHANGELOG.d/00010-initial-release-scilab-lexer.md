## 0.1.0

- Initial Dart lane of the scilab-lexer identity: `tokenizeScilab` /
  `tryTokenizeScilab` / `createScilabLexer` over the compiled
  `scilab.tokens` grammar (`lib/src/_grammar.dart`, generated with
  the in-lane emitter) plus the four hooks: quote-protection
  pre-tokenize (transpose-vs-string disambiguation with `//` and
  `/* */` comment passthrough), placeholder restoration,
  double-quoted escape collapse, and paren-interior newline
  dropping (saturating on unbalanced `)`). Verbatim grammar-name
  types and a final `EOF`; lex failures throw the engine's
  structured `LexerError`, and the nullable twin returns `null`
  instead.
- Tests mirror the reference suite (all 35 tests), 9 seeded property
  tests over the in-lane `pbt` package, and a 2,000-iteration hostile
  fuzz campaign over the in-lane `fuzz` package; 100% line and
  function coverage.
