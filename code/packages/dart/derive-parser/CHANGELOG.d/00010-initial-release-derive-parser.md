## 0.1.0

- Initial Dart lane of the derive-parser identity: `parseDerive` /
  `tryParseDerive` / `createDeriveParser` over the compiled
  `derive.grammar` grammar (`lib/src/_grammar.dart`, generated with
  the in-lane emitter), tokenizing through the Dart derive-lexer
  lane and parsing with a `maxRuleDepth` of 200. Trees root at
  `program`; lex failures throw the engine's `LexerError`, syntax
  failures throw `GrammarParseError`, and the nullable twin returns
  `null` for either.
- Tests mirror the reference suite (all 16 tests), 9 seeded
  property tests over the in-lane `pbt` package, and a
  2,000-iteration hostile fuzz campaign over the in-lane `fuzz`
  package; 100% line and function coverage.
