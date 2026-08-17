# Changelog

## 0.1.2 — 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `create_lexer/0` now imports a pre-compiled grammar module (`CodingAdventures.XmlLexer.Grammar`) instead of `File.read!`-ing `xml.tokens` from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published Hex package does not ship, so `mix deps.get` + first use would raise `File.Error` (enoent).

## 0.1.1 — 2026-08-07

### Fixed
- **`xml.tokens` is now lookaround-free.** `COMMENT_TEXT`/`CDATA_TEXT`/
  `PI_TEXT` no longer use negative-lookahead regex; see
  `coding-adventures-xml-parser`'s (Rust) CHANGELOG for the full rationale.
  One behavior change: a comment/CDATA body containing the delimiter's
  ambiguous character (e.g. a lone `-` inside `<!-- a-b -->`) now surfaces
  as several adjacent `COMMENT_TEXT`/`CDATA_TEXT` tokens instead of one;
  concatenate them to get the original text (see the updated "comment with
  dashes"/"CDATA with single bracket" tests). `CHAR_REF` is now produced by
  two aliased rules (`CHAR_REF_HEX`/`CHAR_REF_DEC`) instead of one rule
  with a top-level `A|B` pattern — no observable change to `CHAR_REF`
  tokens themselves.
- **Fixed a latent PI-body mis-tokenization bug**: `<?t a?b?>` had the `b`
  after the bare `?` wrongly re-tokenized as a second `PI_TARGET` instead
  of `PI_TEXT`, because the old single `pi` group offered `PI_TARGET`'s
  pattern for the whole PI body, not just the first token.
  `xml_on_token/2` now swaps from the `pi` group to a new `pi_body` group
  the instant `PI_TARGET` matches, so its pattern is never re-offered.
  Covered by a new regression test.

## 0.1.0 — 2026-03-21

### Added
- `XmlLexer.tokenize/1` — tokenize XML source code using pattern groups and callback hooks
- `XmlLexer.create_lexer/0` — parse xml.tokens grammar
- `XmlLexer.xml_on_token/2` — callback function that drives group switching for context-sensitive XML lexing
- Grammar caching via `persistent_term` for repeated use
- 30+ tests covering basic tags, attributes, self-closing tags, comments, CDATA sections, processing instructions, entity references, nested/mixed content, edge cases, and callback actions
