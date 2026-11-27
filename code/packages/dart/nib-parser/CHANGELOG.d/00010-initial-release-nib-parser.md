## 0.1.0

- Initial Dart lane of the nib-parser identity: `parseNib` /
  `tryParseNib` / `parseNibTokens` / `tryParseNibTokens` plus the
  `createNibParser` / `createNibParserFromTokens` factories over the
  compiled `nib.grammar` grammar (`lib/src/_grammar.dart`, generated
  with the in-lane emitter), tokenizing through the Dart nib-lexer
  lane and parsing with a `maxRuleDepth` of 200. Lex failures throw
  the engine's `LexerError`, syntax failures throw
  `GrammarParseError`, and the nullable twins return `null` for any
  failure.
- Tests mirror the reference suite (all 7 tests, with the 16/17
  nesting boundary re-measured on this engine), plus 8 seeded
  property tests over the in-lane `pbt` package and a 2,000-iteration
  hostile fuzz campaign over the in-lane `fuzz` package; 100% line
  and function coverage.
