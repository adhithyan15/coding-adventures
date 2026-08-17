# Changelog

All notable changes to the XML Lexer package will be documented in this file.

## [0.1.2] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `createXMLLexer` now imports a pre-compiled `_grammar.ts` instead of `readFileSync`-ing `xml.tokens` from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published npm package does not ship, so `npm install` + first use would throw `ENOENT`.

## [0.1.1] - 2026-08-07

### Fixed

- **`xml.tokens` is now lookaround-free.** `COMMENT_TEXT`/`CDATA_TEXT`/
  `PI_TEXT` no longer use negative-lookahead regex; see
  `coding-adventures-xml-parser`'s CHANGELOG for the full rationale. One
  behavior change: a comment/CDATA body containing the delimiter's
  ambiguous character (e.g. a lone `-` inside `<!-- a-b -->`) now surfaces
  as several adjacent `COMMENT_TEXT`/`CDATA_TEXT` tokens instead of one;
  concatenate them to get the original text (see the updated dashes/
  brackets tests). `CHAR_REF` is now produced by two aliased rules
  (`CHAR_REF_HEX`/`CHAR_REF_DEC`) instead of one rule with a top-level
  `A|B` pattern — no observable change to `CHAR_REF` tokens themselves.
- **Fixed a latent PI-body mis-tokenization bug**: `<?t a?b?>` had the `b`
  after the bare `?` wrongly re-tokenized as a second `PI_TARGET` instead
  of `PI_TEXT`, because the old single `pi` group offered `PI_TARGET`'s
  pattern for the whole PI body, not just the first token. The on-token
  callback now swaps from the `pi` group to a new `pi_body` group the
  instant `PI_TARGET` matches, so its pattern is never re-offered. Covered
  by a new regression test.

## [0.1.0] - 2026-03-21

### Added

- Initial release of the XML lexer for TypeScript.
- `tokenizeXML()` function that tokenizes XML text using pattern groups and the on-token callback.
- `createXMLLexer()` function that returns a configured `GrammarLexer` instance.
- `xmlOnToken()` callback that drives context-sensitive group switching for XML tokenization.
- Loads `xml.tokens` grammar file defining 5 pattern groups: default, tag, comment, cdata, pi.
- Full support for XML elements: open tags, close tags, self-closing tags, namespaced tags.
- Full support for attributes: double-quoted and single-quoted values.
- Full support for comments (`<!-- -->`), CDATA sections (`<![CDATA[ ]]>`), and processing instructions (`<? ?>`).
- Entity references (`&amp;`) and character references (`&#65;`, `&#x41;`).
- Whitespace preservation inside comments, CDATA, and PIs (skip patterns disabled via callback).
- Comprehensive test suite covering basic tags, attributes, comments, CDATA, PIs, entity references, nested/mixed content, edge cases, and position tracking.
