## 0.1.0

- Initial Dart lane of the r-parser identity: `parseR` /
  `tryParseR` / `createRParser` over the compiled `r.grammar`
  grammar (`lib/src/_grammar.dart`, generated with the in-lane
  emitter), tokenizing through the Dart r-lexer lane and parsing
  with a `maxRuleDepth` of 200. Trees root at `program` with the
  S-shared rule names; lex failures throw the engine's `LexerError`,
  syntax failures throw `GrammarParseError`, and the nullable twin
  returns `null` for either.
- Tests mirror the reference suite (all 16 tests), 9 seeded
  property tests over the in-lane `pbt` package, and a
  2,000-iteration hostile fuzz campaign over the in-lane `fuzz`
  package; 100% line and function coverage.
