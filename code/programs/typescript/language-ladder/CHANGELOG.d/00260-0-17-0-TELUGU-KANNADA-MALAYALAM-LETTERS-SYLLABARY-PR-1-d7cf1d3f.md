## 0.17.0 — Telugu, Kannada & Malayalam letters (syllabary, PR 1)

- **The three Dravidian scripts now have letters.** Browse and Practice covered
  only Arabic / Devanagari / Tamil; Telugu, Kannada, and Malayalam — the tail of
  the language chain — had none. They're **abugidas**, so each "letter" is a
  syllable: a base consonant carries an inherent *a*, and a vowel sign turns it
  into ka → ki → ku, kha → khi → khu. All three now appear as Browse tabs (350
  Telugu / 350 Kannada / 360 Malayalam syllables) and drill in Practice, reusing
  the existing letter engine unchanged (a syllable is just a `Letter`).
- **Generated from Unicode, not hand-typed.** New
  `data/scripts/generate_syllabary.py` composes every syllable from Unicode code
  points, taking each base consonant / vowel-sign's identity and ISO-15919
  romanization from its official Unicode character name (`TELUGU LETTER KA`, …) —
  a letter it can't name from the standard is skipped, never guessed. The three
  `*.json` files are its regenerable output; the generator is the provenance.
- **Recognition only — no fabricated stroke order.** These carry `strokeOrder:
  []` (their ductus is a separate, source-gated effort, still paused). The Browse
  detail now **hides the "Write it — stroke order" section when there is none**,
  rather than showing an empty one — so we never imply data we don't have. The
  grounded consonant⊕vowel-sign decomposition (`క ka + ి "i" sign`) still shows.
- 10 new tests (238 total) grounding the glyphs to code points, with controls
  that bite: `ka` must equal the block's KA code point, `ki` must be KA + the
  i-sign, and every syllable's `strokeOrder` must be `[]`. Verified in a real
  browser — Telugu and Malayalam grids render real glyphs, no tofu. Slowly
  unlocking the syllables one consonant at a time is the next slice (PR 2).

