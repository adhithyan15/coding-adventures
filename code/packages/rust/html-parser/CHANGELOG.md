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
- Parser-driven tokenizer handoff for RCDATA, RAWTEXT, script data, and
  PLAINTEXT elements, preserving text-mode DOM content instead of lexing it as
  ordinary data-state markup.
