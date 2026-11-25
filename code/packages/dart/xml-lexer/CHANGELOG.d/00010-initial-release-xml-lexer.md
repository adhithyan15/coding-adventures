## 0.1.0

- Initial Dart lane of the xml-lexer identity: `tokenizeXml` /
  `createXmlLexer` over the compiled `xml.tokens` grammar
  (`lib/src/_grammar.dart`, generated with the in-lane emitter) plus
  the `xmlOnToken` group-stack callback — tags, attributes, entities,
  character references, comments, CDATA, and processing instructions
  with verbatim grammar-name types and a final `EOF`. Lex failures
  throw the engine's structured `LexerError`. First engine-driven
  (`GrammarLexer`) Dart lexer lane.
- Tests mirror the reference suite (all 38 tests) and add the
  untested-corner matrix, 8 seeded property tests over the in-lane
  `pbt` package, and a 2,000-iteration hostile fuzz campaign over
  the in-lane `fuzz` package; 100% line and function coverage.
