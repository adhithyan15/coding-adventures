## 0.1.0

- Initial Dart lane of the fsharp-lexer identity, and the first
  versioned Dart lane: `tokenizeFsharp` / `createFsharpLexer` over
  15 compiled `fsharp*.tokens` grammars
  (`lib/src/_grammar.dart`, one per supported version, generated
  with the grammar-tools compiler), with `resolveFsharpVersion`
  (`''` selects the default `'10'`) and `supportedFsharpVersions`
  mirroring the reference constants.
- Tests mirror the reference suite (all 3 tests) plus 7 lane
  extras, 9 seeded property tests over the in-lane `pbt` package,
  and a 2,000-iteration hostile fuzz campaign over the in-lane
  `fuzz` package (every input runs under all 15 versions);
  100% line and function coverage.
