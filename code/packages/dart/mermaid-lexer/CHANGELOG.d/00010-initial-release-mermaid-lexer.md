## 0.1.0

- Initial Dart lane of the mermaid-lexer identity: 34 per-family
  factories, 9 throwing one-shots, and 27 nullable twins over 34
  compiled `mermaid/*.tokens` grammars
  (`lib/src/_grammar.dart`, generated with the grammar-tools
  compiler), including the sequence keyword-normalization and
  state line-filter post-processors.
- Tests mirror the reference suite (all 135 tests) plus 2 lane
  extras, 8 seeded property tests over the in-lane `pbt` package,
  and a 2,000-iteration hostile fuzz campaign over the in-lane
  `fuzz` package (every input runs under all 34 families);
  100% line and function coverage.
- Compiler fix (same branch, separate commit): the Dart
  `.tokens` parser normalizes Rust-regex syntax Dart's `RegExp`
  rejects — `(?i:...)` groups become explicit case pairs and
  Annex-B-only `\-` becomes `\x2D` — with clear errors for other
  inline flags; see the grammar-tools CHANGELOG.
