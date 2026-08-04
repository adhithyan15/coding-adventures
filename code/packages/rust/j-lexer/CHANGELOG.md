# Changelog

## [0.1.0] - 2026-07-12

### Added

- Initial grammar-driven Rust J tokenizer (MA06 §6, task MA-6b).
- Statically linked compiled token grammar (`code/grammars/j/j.tokens`),
  covering the historical-core subset fixed by MA-6a: dense numeric arrays,
  the primitive verb glyphs (including the ASCII digraphs `<.`/`>.`), the
  six comparison glyphs, the two adverbs (reduce/scan), the one conjunction
  (`@` compose), assignment, parenthesised grouping, and `NB.` line
  comments.
- Longest-match-first digraph ordering for every `.`/`:`-suffixed sibling
  of a base character (`<.`/`<:`/`<`, `>.`/`>:`/`>`, `~:`, `=.`/`=:`/`=`,
  `i.`), applied far more pervasively than APL's own single precedent
  (`∘.`), since J needs it for roughly a dozen base characters.
- A dedicated regression test guarding MA06 §1's single most common
  APL-to-J transliteration mistake: `/` is the reduce adverb, not division
  (`%` is division).
- A leading-underscore negative-literal convention (`_5`) for `NUMBER`,
  since MA06 §4 does not spell out a literal syntax — documented in
  `j.tokens`'s own header, with a bare `_`/`__` (infinity) structurally
  excluded from this cut rather than merely documented as deferred.
