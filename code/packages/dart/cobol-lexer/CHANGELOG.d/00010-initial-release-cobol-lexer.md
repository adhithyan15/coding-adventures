## 0.1.0

- Initial Dart lane of the cobol-lexer identity: `tokenizeCobol` /
  `tryTokenizeCobol` / `createCobolLexer` over the compiled
  `cobol.tokens` grammar (`lib/src/_grammar.dart`, generated with
  the in-lane emitter) plus the `stripCobolColumns` pre-tokenize
  hook (80-column card → free-form code). PICTURE strings ride the
  grammar's declarative `picture` mode transition — no hook needed.
  Verbatim grammar-name types and a final `EOF`; lex failures throw
  the engine's structured `LexerError`, and the nullable twin
  returns `null` instead.
- Tests mirror the reference suite (all 16 tests), 9 seeded
  property tests over the in-lane `pbt` package, and a
  2,000-iteration hostile fuzz campaign over the in-lane `fuzz`
  package; 100% line and function coverage.
