## 0.1.0

- Initial Dart lane of the fsharp-parser identity, and the first
  versioned Dart parser lane: `parseFsharp` / `tryParseFsharp` /
  `createFsharpParser` over 15 compiled `fsharp*.grammar` grammars
  (`lib/src/_grammar.dart`, generated with the grammar-tools
  compiler), tokenizing through the Dart fsharp-lexer lane with
  `resolveFsharpParserVersion` (`''` selects the default `'10'`)
  and parsing with a `maxRuleDepth` of 200. Trees root at
  `compilation_unit`.
- Tests mirror the reference suite (all 5 tests) plus 6 lane
  extras, 9 seeded property tests over the in-lane `pbt` package,
  and a 2,000-iteration hostile fuzz campaign over the in-lane
  `fuzz` package (every input runs under all 15 versions);
  100% line and function coverage.
