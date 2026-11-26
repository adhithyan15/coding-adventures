## 0.1.0

- Initial Dart lane of the oct-parser identity: `parseOct` /
  `tryParseOct` plus the `createOctParser` /
  `createOctParserFromTokens` factories over the compiled
  `oct.grammar` grammar (`lib/src/_grammar.dart`, generated with
  the in-lane emitter), tokenizing through the Dart oct-lexer lane
  and parsing with a `maxRuleDepth` of 200. Lex failures throw the
  engine's `LexerError`, syntax failures throw `GrammarParseError`,
  and the nullable twin returns `null` for any failure.
- Tests mirror the reference suite (all 14 tests, with the 20/21
  nesting boundary re-measured on this engine), plus 8 seeded
  property tests over the in-lane `pbt` package and a 2,000-iteration
  hostile fuzz campaign over the in-lane `fuzz` package; 100% line
  and function coverage.
