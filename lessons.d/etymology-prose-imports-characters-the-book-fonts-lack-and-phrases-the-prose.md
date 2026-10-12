---
category: Testing & coverage
---

# Etymology prose imports characters the book fonts lack and phrases the prose gates count

Filling 1,077 German etymologies from EtymDB 2.1, every lesson-level gate was
green and the full `vitest run` still failed four tests, all caused by text
copied from the dataset's forms:

- **Glyphs.** Proto-Germanic *\*tungǭ* (U+01ED, o with ogonek and macron) and
  Latin *būtȳrum* (U+0233) are not in the German book's main font, so
  `glyph-coverage` failed on 13 chapters. Fix: write the older notation
  *\*tungōn* and drop the Latin vowel-length mark. Hindi *cā̃po* (macron vowel
  plus U+0303) made German a "long nasal vowel" track in `nasal-macron`; it is
  now *chāmpo*.
- **Prose gates.** "what you are used to" is an `is-used-for` rule statement to
  `info-dump`, and the corpus ceiling has no headroom. Gothic *garaihts*,
  "just", is a banned word, and "the course of a life" contains the banned
  cross-volume string. Gloss text inside an etymology block is still block
  prose.

Do: before the full suite, grep the new text for characters outside the
track's existing repertoire and run `glyph-coverage`, `nasal-macron`,
`info-dump` and `banned-words` alone; they take seconds. Glosses and
reconstructions are not exempt from any gate.
