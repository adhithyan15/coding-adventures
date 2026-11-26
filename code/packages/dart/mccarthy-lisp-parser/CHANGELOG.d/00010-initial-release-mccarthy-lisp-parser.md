## 0.1.0

- Initial Dart lane of the mccarthy-lisp-parser identity:
  `parseMcCarthy` / `tryParseMcCarthy` / `parseMcCarthyToCst` /
  `createMcCarthyParserFromTokens` / `extractProgram` over the
  compiled `mccarthy_lisp.grammar` grammar
  (`lib/src/_grammar.dart`, generated with the in-lane emitter),
  tokenizing through the Dart mccarthy-lisp-lexer lane. Programs
  lower to the typed `LispExpr` tree (`LispNil`, `LispSymbol`,
  `LispInt`, `LispCons`); all failures throw the structured
  `McCarthyParseError`. The three reference depth guards are
  mirrored (`maxParenDepth` 64, engine cap with list 59/60 and quote
  88/89 boundaries, `maxAstDepth` 256).
- Tests mirror the reference suite (22 unit, 6 depth-guard, 26
  integration), 11 seeded property tests over the in-lane `pbt`
  package, and a 2,000-iteration hostile fuzz campaign over the
  in-lane `fuzz` package; 100% line and function coverage.
