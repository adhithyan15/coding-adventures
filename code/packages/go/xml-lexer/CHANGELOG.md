# Changelog

All notable changes to the xml-lexer package will be documented in this file.

## [0.2.1] - 2026-08-07

### Fixed

- **Removed the `rewriteGroup`/`goCompatibleGrammar` runtime pattern-rewrite
  machinery.** It rewrote the embedded grammar's `COMMENT_TEXT`/`CDATA_TEXT`/
  `PI_TEXT` patterns to Go-`regexp`-compatible equivalents (Go's `regexp`
  package has no lookaround) every time a lexer was created. The canonical
  `code/grammars/xml/xml.tokens` these patterns are compiled from is now
  itself lookaround-free (see `coding-adventures-xml-parser`'s CHANGELOG for
  the full rationale), so the rewrite is unnecessary — `CreateXmlLexer` now
  uses `TokenGrammarData` directly. `mergeAdjacentTokens`/`TokenizeXml`'s
  merge-back-to-one-token behavior is unchanged and still needed, since the
  lookaround-free encoding still produces multiple adjacent tokens per
  comment/CDATA run internally.
- **Fixed a latent PI-body mis-tokenization bug**: `<?t a?b?>` had the `b`
  after the bare `?` wrongly re-tokenized as a second `PI_TARGET` instead of
  `PI_TEXT`, because the old single `pi` group offered `PI_TARGET`'s pattern
  for the whole PI body, not just the first token. `XmlOnToken` now swaps
  from the `pi` group to a new `pi_body` group the instant `PI_TARGET`
  matches, so its pattern is never re-offered. Covered by
  `TestPIBodyWithBareQuestionMarkNotRetokenizedAsTarget`.

## [0.2.0] - 2026-07-13

### Changed

- The token grammar is now embedded at compile time as native Go data in
  `grammar_data.go` and consumed directly by the loader; the package no longer
  reads `code/grammars/**` at run time via `runtime.Caller`. This drops the
  filesystem capability (empty `required_capabilities.json`, `gen_capabilities.go`
  removed), lets the package build and run standalone, and fixes the previously
  dead embedded grammar (the old `_grammar.go` name is ignored by the Go
  toolchain because of its leading underscore).

## [0.1.0] - 2026-03-21

### Added
- Initial implementation of the XML lexer wrapper around the grammar-driven lexer
- `XmlOnToken` callback function for context-sensitive pattern group switching:
  - Pushes "tag" group on `OPEN_TAG_START` or `CLOSE_TAG_START`
  - Pops group on `TAG_CLOSE` or `SELF_CLOSE`
  - Pushes "comment" group and disables skip on `COMMENT_START`
  - Pops and re-enables skip on `COMMENT_END`
  - Same push/pop + skip toggle pattern for CDATA and PI groups
- `CreateXmlLexer(source string)` factory function for creating reusable lexer instances
- `TokenizeXml(source string)` convenience function for one-shot tokenization
- Grammar file loading via `runtime.Caller(0)` for location-independent operation
- Support for all XML token types across 5 pattern groups:
  - Default: TEXT, ENTITY_REF, CHAR_REF, tag/comment/cdata/pi openers
  - Tag: TAG_NAME, ATTR_EQUALS, ATTR_VALUE, TAG_CLOSE, SELF_CLOSE
  - Comment: COMMENT_TEXT, COMMENT_END
  - CDATA: CDATA_TEXT, CDATA_END
  - PI: PI_TARGET, PI_TEXT, PI_END
- Comprehensive test suite covering:
  - Basic elements (simple, namespaced, empty, self-closing)
  - Attributes (double-quoted, single-quoted, multiple, on self-closing)
  - Comments (simple, whitespace preservation, dashes, between elements, empty)
  - CDATA sections (simple, angle brackets, whitespace, single bracket, empty)
  - Processing instructions (XML declaration, stylesheet PI)
  - Entity references (named, decimal char ref, hex char ref, multiple)
  - Nested and mixed content (nesting, mixed text, full document, CDATA inside element)
  - Edge cases (empty input, text only, whitespace skipping, EOF, line/column, deep nesting)
