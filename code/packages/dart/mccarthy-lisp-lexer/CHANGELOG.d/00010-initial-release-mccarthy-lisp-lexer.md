## 0.1.0

- Initial Dart lane of the mccarthy-lisp-lexer identity:
  `tokenizeMcCarthy` / `createMcCarthyLexer` /
  `mccarthyTokenGrammarSpec` over the compiled
  `mccarthy_lisp.tokens` grammar (`lib/src/_grammar.dart`,
  generated with the grammar-tools compiler).
- Tests mirror the reference suite (all 11 tests) plus 3 lane
  extras, 7 seeded property tests over the in-lane `pbt` package,
  and a 2,000-iteration hostile fuzz campaign over the in-lane
  `fuzz` package; 100% line and function coverage.
