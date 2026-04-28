# Changelog

All notable changes to the `coding-adventures-html-lexer` crate will be
documented in this file.

## [0.1.0] - 2026-04-23

### Added
- Initial Rust HTML lexer package.
- Statically linked HTML skeleton state machine over the generic
  `state-machine-tokenizer` runtime.
- Build-time `html-skeleton.lexer.states.toml` authoring artifact.
- Build-time `html1.lexer.states.toml` authoring artifact for the Mosaic-era
  HTML compatibility floor, covering tags, attributes, comments, doctypes, and
  EOF recovery without runtime TOML loading.
- Checked-in generated Rust module for the HTML skeleton lexer definition, so
  the crate links static source instead of loading TOML at runtime.
- Fixture-backed tests proving the generated lexer definition and emitted
  runtime tokens stay aligned.

### Changed
- Switched the stable `create_html_lexer` and `lex_html` API over from the
  bootstrap skeleton to the generated `html1` compatibility-floor lexer.
- Kept the bootstrap skeleton helpers available for focused comparisons while
  the default wrapper now exercises attributes, comments, doctypes, and
  Mosaic-era fixtures.

### Added
- Repo-native JSON conformance fixture suites for the bootstrap skeleton and the
  `html1` compatibility-floor lexer.
- A shared Rust conformance harness that runs the fixture suites against both
  explicit generated constructors and the default wrapper API.
- Documentation for the fixture schema and the planned WHATWG/WPT normalization
  path.
- A first raw html5lib-style tokenizer smoke file plus a Rust normalizer that
  lowers supported upstream cases into Venture's portable conformance schema.
- A checked-in importer script and generated normalized `html5lib-smoke.json`
  corpus so broader upstream-style cases can be regenerated instead of
  hand-maintained in Rust tests.
- RCDATA and `lastStartTag` support in the html5lib importer, with normalized
  fixture metadata that seeds the first executable Rust RCDATA cases while the
  importer still records unsupported upstream states separately.
- RAWTEXT support in the authored `html1` machine, the html5lib importer, and
  the Rust conformance harness, so seeded style-like tokenizer cases now
  execute through the generated static wrapper instead of being skipped.
- Named character reference support for the current generated Rust lexer in
  data, RCDATA, and attribute values, covering `amp`, `lt`, `gt`, `quot`, and
  `apos` as the first shared entity set.
- Decimal and hexadecimal numeric character references in data, RCDATA, and
  attribute values, including replacement-character fallback for null or
  invalid scalar values.
