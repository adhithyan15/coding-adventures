# Changelog

All notable changes to `coding_adventures_xml_lexer` will be documented in this file.

## [0.1.2] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `create_xml_lexer` now loads the pre-compiled `_grammar.rb` (via `GrammarTools::CompiledLoader`) instead of reading and parsing `xml.tokens` from `code/grammars/` on every call. The old code walked out of the installed gem's own directory to a monorepo-relative path that a published gem does not ship, so a real `gem install` + first use would raise `Errno::ENOENT`.

## [0.1.1] - 2026-08-07

### Fixed
- **`xml.tokens` is now lookaround-free.** `COMMENT_TEXT`/`CDATA_TEXT`/
  `PI_TEXT` no longer use negative-lookahead regex; see
  `coding-adventures-xml-parser`'s (Rust) CHANGELOG for the full rationale.
  One behavior change: a comment/CDATA body containing the delimiter's
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
  pattern for the whole PI body, not just the first token. The on-token
  callback now swaps from the `pi` group to a new `pi_body` group the
  instant `PI_TARGET` matches, so its pattern is never re-offered. Covered
  by `test_pi_body_with_bare_question_mark_not_retokenized_as_target`.

## [0.1.0] - 2026-03-21

### Added
- Initial release
- `CodingAdventures::XmlLexer.tokenize(source)` method that tokenizes XML text
- `CodingAdventures::XmlLexer.create_xml_lexer(source)` factory for configured GrammarLexer
- `XML_ON_TOKEN` callback proc for pattern group switching
- Loads `xml.tokens` grammar file and delegates to `GrammarLexer` with on-token callback
- Pattern group switching for context-sensitive lexing:
  - **default** group: TEXT, ENTITY_REF, CHAR_REF, tag/comment/CDATA/PI openers
  - **tag** group: TAG_NAME, ATTR_EQUALS, ATTR_VALUE, TAG_CLOSE, SELF_CLOSE
  - **comment** group: COMMENT_TEXT, COMMENT_END (skip disabled)
  - **cdata** group: CDATA_TEXT, CDATA_END (skip disabled)
  - **pi** group: PI_TARGET, PI_TEXT, PI_END (skip disabled)
- Supports namespace prefixes in tag names (e.g., `ns:tag`)
- Supports single and double quoted attribute values
- Full test suite with SimpleCov coverage >= 80%
