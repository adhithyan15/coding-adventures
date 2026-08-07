# Changelog -- XMLLexer (Swift)

## [0.2.1] -- 2026-08-07

### Fixed
- **`xml.tokens` is now lookaround-free.** `COMMENT_TEXT`/`CDATA_TEXT`/
  `PI_TEXT` no longer use negative-lookahead regex; see
  `coding-adventures-xml-parser`'s (Rust) CHANGELOG for the full rationale.
  One behavior change: a comment/CDATA body containing the delimiter's
  ambiguous character (e.g. a lone `-` inside `<!-- a-b -->`) now surfaces
  as several adjacent `COMMENT_TEXT`/`CDATA_TEXT` tokens instead of one;
  concatenate them to get the original text (see the updated
  `testSingleDashesInsideCommentsAllowed`/`testSingleBracketsInsideCDATA`).
  `CHAR_REF` is now produced by two aliased rules (`CHAR_REF_HEX`/
  `CHAR_REF_DEC`) instead of one rule with a top-level `A|B` pattern — no
  observable change to `CHAR_REF` tokens themselves.
- **Fixed a latent PI-body mis-tokenization bug**: `<?t a?b?>` had the `b`
  after the bare `?` wrongly re-tokenized as a second `PI_TARGET` instead
  of `PI_TEXT`, because the old single `pi` group offered `PI_TARGET`'s
  pattern for the whole PI body, not just the first token. `xmlOnToken`
  now swaps from the `pi` group to a new `pi_body` group the instant
  `PI_TARGET` matches, so its pattern is never re-offered. Covered by
  `testPIBodyWithBareQuestionMarkNotRetokenizedAsTarget`.

## [0.2.0] -- 2026-07-13

### Changed
- `loadGrammar()` now returns the grammar embedded at compile time in the
  generated `_Grammar.swift` instead of reading `code/grammars/**` via
  `#filePath` at run time. The lexer/parser no longer depends on the
  monorepo layout and works when published standalone. Grammar source of
  truth is unchanged; regenerate via `swift/grammar-tools`' `grammar-tools-embed`.

## [0.1.1] -- 2026-07-12

### Fixed
- `loadGrammar()` now reads the grammar from `grammars/xml/xml.tokens`.
  PR #7475 moved every grammar into a per-grammar `code/grammars/<name>/`
  subdirectory; this lexer still used the old flat `grammars/xml.tokens`
  path, so every test failed with "The file doesn't exist."

## [0.1.0] -- 2026-04-12

### Added

- Initial implementation of `XMLLexer`.
- `tokenize(_:)` -- tokenizes XML source using `xml.tokens`.
- `loadGrammar()` -- loads and parses the XML token grammar.
- Group stack parsing: default, tag, cdata, comment, pi.
- Comprehensive XCTest suite.
- `BUILD` and `BUILD_windows` scripts.
