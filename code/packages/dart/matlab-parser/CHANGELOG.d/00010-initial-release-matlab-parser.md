## 0.1.0

- Initial Dart lane of the matlab-parser identity: `parseMatlab` /
  `tryParseMatlab` / `createMatlabParser` over the compiled
  `matlab.grammar` grammar (`lib/src/_grammar.dart`, generated with
  the in-lane emitter), tokenizing through the Dart matlab-lexer
  lane and parsing with a `maxRuleDepth` of 200 plus the
  `retagIndexEnd` pre-parse hook (bracketed `end` becomes `NAME`).
  Trees root at `program`; lex failures throw the engine's
  `LexerError`, syntax failures throw `GrammarParseError`, and the
  nullable twin returns `null` for either.
- Tests mirror the reference suite (all 19 tests), 9 seeded
  property tests over the in-lane `pbt` package, and a
  2,000-iteration hostile fuzz campaign over the in-lane `fuzz`
  package; 100% line and function coverage.
