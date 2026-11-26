## 0.1.0

- Initial Dart lane of the macrooct-lexer identity:
  `tokenizeMacrooct` / `tryTokenizeMacrooct` /
  `createMacrooctLexer` over the compiled `macrooct.tokens`
  grammar (`lib/src/_grammar.dart`, generated with the in-lane
  emitter) plus the keyword-promotion post-pass (`KEYWORD("fn")`
  becomes `fn("fn")`, exactly like the reference). The factory
  returns the raw engine; only the one-shots promote. Verbatim
  grammar-name types and a final `EOF`; lex failures throw the
  engine's structured `LexerError`, and the nullable twin returns
  `null` instead.
- Tests mirror the reference suite (all 11 tests, including the
  PREP01 drift-guard and compiled-grammar conformance tests via the
  in-lane parser), 9 seeded property tests over the in-lane `pbt`
  package, and a 2,000-iteration hostile fuzz campaign over the
  in-lane `fuzz` package; 100% line and function coverage.
