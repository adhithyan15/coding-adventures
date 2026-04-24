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
