## 0.1.0

- Initial Dart lane of the apl-parser identity: `parseApl` /
  `tryParseApl` / `createAplParser` over the compiled `apl.grammar`
  grammar (`lib/src/_grammar.dart`, generated with the in-lane
  emitter), tokenizing through the Dart apl-lexer lane and parsing
  with a `maxRuleDepth` of 100. Trees root at `program`; lex
  failures throw the engine's `LexerError`, syntax failures throw
  `GrammarParseError`, and the nullable twin returns `null` for
  either.
- Tests mirror the reference suite (all 22 tests), 9 seeded
  property tests over the in-lane `pbt` package, and a
  2,000-iteration hostile fuzz campaign over the in-lane `fuzz`
  package; 100% line and function coverage.
