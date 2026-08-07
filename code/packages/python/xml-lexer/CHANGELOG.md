# Changelog

All notable changes to the xml-lexer package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2026-08-07

### Fixed

- **`xml.tokens` is now lookaround-free.** `COMMENT_TEXT`/`CDATA_TEXT`/
  `PI_TEXT` no longer use negative-lookahead regex; see
  `coding-adventures-xml-parser`'s CHANGELOG for the full rationale. One
  behavior change: a comment/CDATA body containing the delimiter's
  ambiguous character (e.g. a lone `-` inside `<!-- a-b -->`) now surfaces
  as several adjacent `COMMENT_TEXT`/`CDATA_TEXT` tokens instead of one;
  concatenate them to get the original text (see the updated
  `test_comment_with_dashes`/`test_cdata_with_single_bracket`). `CHAR_REF`
  is now produced by two aliased rules (`CHAR_REF_HEX`/`CHAR_REF_DEC`)
  instead of one rule with a top-level `A|B` pattern — no observable change
  to `CHAR_REF` tokens themselves.
- **Fixed a latent PI-body mis-tokenization bug**: `<?t a?b?>` had the `b`
  after the bare `?` wrongly re-tokenized as a second `PI_TARGET` instead
  of `PI_TEXT`, because the old single `pi` group offered `PI_TARGET`'s
  pattern for the whole PI body, not just the first token. `xml_on_token`
  now swaps from the `pi` group to a new `pi_body` group the instant
  `PI_TARGET` matches, so its pattern is never re-offered. Covered by
  `test_pi_body_with_bare_question_mark_not_retokenized_as_target`.

## [0.1.0] - 2026-03-22

### Added
- Initial release of the XML lexer package
- `tokenize_xml()` — tokenize XML text and return a list of tokens
- `create_xml_lexer()` — create a configured GrammarLexer for XML
- `xml_on_token()` — the on-token callback that drives group transitions
- Support for: tags, attributes, self-closing tags, comments, CDATA
  sections, processing instructions, entity references, character references
- First lexer wrapper to use pattern groups and callback hooks
