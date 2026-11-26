## 0.1.0

- Initial Dart lane of the j-parser identity: `parseJ` /
  `tryParseJ` / `createJParser` over the compiled `j.grammar`
  grammar (`lib/src/_grammar.dart`, generated with the in-lane
  emitter), tokenizing through the Dart j-lexer lane and parsing
  with a `maxRuleDepth` of 70. Trees root at `program`; lex
  failures throw the engine's `LexerError`, syntax failures throw
  `GrammarParseError`, and the nullable twin returns `null` for
  either.
- Tests mirror the reference suite (all 33 tests), 9 seeded
  property tests over the in-lane `pbt` package, and a
  2,000-iteration hostile fuzz campaign over the in-lane `fuzz`
  package; 100% line and function coverage.
