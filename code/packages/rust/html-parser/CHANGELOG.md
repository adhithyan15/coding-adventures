# Changelog

All notable changes to the `coding-adventures-html-parser` crate will be
documented in this file.

## [0.1.0] - 2026-05-02

### Added
- Initial HTML parser crate that consumes `coding-adventures-html-lexer` tokens
  and builds a `dom-core` document.
- Stack-of-open-elements tree construction seed with void element handling,
  adjacent text merging, simple implied end tags, and unmatched end-tag
  diagnostics.
