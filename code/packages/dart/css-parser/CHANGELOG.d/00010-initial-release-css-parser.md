## 0.1.0

- Initial Dart lane of the css-parser identity: `parseCss` /
  `createCssParser` over the compiled `css.grammar` grammar
  (`lib/src/_grammar.dart`, generated with the in-lane emitter),
  tokenizing through the Dart css-lexer lane and parsing with a
  `maxRuleDepth` of 170. Trees root at `stylesheet`; lex failures
  throw the engine's `LexerError` and syntax failures throw
  `GrammarParseError`.
- Tests mirror the reference suite (all 13 tests), 9 seeded
  property tests over the in-lane `pbt` package, and a
  2,000-iteration hostile fuzz campaign over the in-lane `fuzz`
  package; 100% line and function coverage.
