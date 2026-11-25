## 0.1.0

- Initial Dart lane of the toml-parser identity: `parseToml` /
  `tryParseToml` / `createTomlParser` / `tryCreateTomlParser` over
  the compiled `toml.grammar` grammar
  (`lib/src/_grammar.dart`, generated with the grammar-tools
  compiler), tokenizing through the Dart toml-lexer lane and
  parsing with a `maxRuleDepth` of 165. Trees root at `document`.
- Failures surface as the payload-blind `TomlParseError`
  (`lexical` / `syntax`, same messages as the reference);
  the nullable twins return `null` instead of throwing.
- Tests mirror the reference suite (all 26 tests), 8 seeded
  property tests over the in-lane `pbt` package, and a
  2,000-iteration hostile fuzz campaign over the in-lane `fuzz`
  package; 100% line and function coverage.
