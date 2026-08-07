# Changelog

All notable changes to this project will be documented in this file.

## [0.1.1] - 2026-08-07

### Fixed

- **Now compiled from the canonical `code/grammars/xml/xml.tokens`** instead
  of a Rust-specific `xml_rust.tokens` fork that existed only to avoid regex
  lookaround (unsupported by the `regex` crate). The canonical file was
  rewritten to avoid lookaround using the portable end-delimiter-first
  technique; see `coding-adventures-xml-parser`'s CHANGELOG for the full
  rationale. One behavior change: a comment/CDATA body containing the
  delimiter's ambiguous character (e.g. a lone `-` inside `<!-- a-b -->`)
  now surfaces as several adjacent `COMMENT_TEXT`/`CDATA_TEXT` tokens
  instead of one; concatenate them to get the original text (see the
  updated `test_comment_with_dashes`/`test_cdata_with_single_bracket`).
- **Fixed a latent PI-body mis-tokenization bug**: `<?t a?b?>` had the `b`
  after the bare `?` wrongly re-tokenized as a second `PI_TARGET` instead
  of `PI_TEXT`, because `PI_TARGET`'s pattern stayed on offer for the whole
  PI body. `xml_on_token` now swaps from the `pi` group to a new `pi_body`
  group the instant `PI_TARGET` matches, so its pattern is never re-offered.
  Covered by `test_pi_body_with_bare_question_mark_not_retokenized_as_target`.

## [0.1.0] - 2026-03-21

### Added

- Initial release of the XML lexer crate.
- `create_xml_lexer()` factory function returning a `GrammarLexer` with the XML on-token callback registered.
- `tokenize_xml()` convenience function returning `Vec<Token>` directly.
- `xml_on_token()` callback function that drives context-sensitive lexing via pattern group transitions.
- Loads the `xml.tokens` grammar file at runtime from the shared `grammars/` directory.
- Supports 5 pattern groups: default (text/entities), tag (names/attributes), comment, cdata, and pi.
- Callback pushes/pops groups on tag/comment/CDATA/PI boundaries and toggles skip for whitespace-significant groups.
- 28 unit tests covering basic tags, namespaces, self-closing tags, attributes (single/double quoted), comments (with whitespace preservation and dashes), CDATA sections (with angle brackets and single brackets), processing instructions, entity references (named, decimal, hex), nested elements, mixed content, full documents, edge cases (empty input, text only, whitespace skipping, EOF), and deeply nested structures.
