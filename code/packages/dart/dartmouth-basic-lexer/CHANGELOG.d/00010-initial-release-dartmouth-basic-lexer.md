## 0.1.0

- Initial Dart lane of the dartmouth-basic-lexer identity:
  `tokenizeDartmouthBasic` / `createDartmouthBasicLexer` over the
  compiled `dartmouth_basic.tokens` grammar
  (`lib/src/_grammar.dart`, generated with the in-lane emitter)
  plus the two post-tokenize hooks: line-number relabeling
  (`relabelLineNumbers`) and REM suppression
  (`suppressRemContent`), exported like the reference. Verbatim
  grammar-name types; lex failures throw the engine's structured
  `LexerError`.
- Tests mirror the reference suite (all 52 tests, plus the
  REM-at-EOF edge proven identical against the Rust crate), 9
  seeded property tests over the in-lane `pbt` package, and a
  2,000-iteration hostile fuzz campaign over the in-lane `fuzz`
  package; 100% line and function coverage.
