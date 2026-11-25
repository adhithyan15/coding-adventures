## 0.1.0

- Initial Dart lane of the c-parser identity: `parseC` /
  `tryParseC` / `createCParser` / `tryParseCTokens` over the
  compiled `c.grammar` grammar (`lib/src/_grammar.dart`, generated
  with the in-lane emitter), tokenizing through the Dart c-lexer
  lane and parsing with a `maxRuleDepth` of 128. Source inputs go
  through legacy directive-line stripping; the token-input twin
  never strips and enforces the token budget first. Lex failures
  throw the engine's `LexerError`, syntax failures and budget
  refusals throw `GrammarParseError`, and the nullable twins return
  `null` for any failure.
- Tests mirror the reference suite (all 19 tests), 9 seeded
  property tests over the in-lane `pbt` package, and a
  2,000-iteration hostile fuzz campaign over the in-lane `fuzz`
  package; 100% line and function coverage.
