## 0.1.0

- Initial Dart lane of the macsyma-parser identity: `parseMacsyma` /
  `tryParseMacsyma` / `createMacsymaParser` over the compiled
  `macsyma.grammar` grammar (`lib/src/_grammar.dart`, generated with
  the in-lane emitter), tokenizing through the Dart macsyma-lexer
  lane and parsing with a `maxRuleDepth` of 200. Lex failures throw
  the engine's `LexerError`, syntax failures throw
  `GrammarParseError`, and the nullable twin returns `null` for any
  failure.
- Tests mirror the reference suite (all 3 depth-guard tests, with
  the 14/15 nesting boundary re-measured on this engine), plus
  valid-program smoke tests, 8 seeded property tests over the
  in-lane `pbt` package, and a 2,000-iteration hostile fuzz campaign
  over the in-lane `fuzz` package; 100% line and function coverage.
