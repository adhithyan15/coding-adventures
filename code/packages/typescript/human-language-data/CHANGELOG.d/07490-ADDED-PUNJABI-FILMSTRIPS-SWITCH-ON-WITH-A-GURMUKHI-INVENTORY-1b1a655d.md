### Added Punjabi filmstrips switch on with a Gurmukhi inventory

- `DERIVED_FILMSTRIP_SCRIPTS.punjabi = "gurmukhi"`. Punjabi's 29 writing
  lessons whose letters now have a cited ductus print a filmstrip: 25
  single-letter lessons and the letter lists ਸ · ਤ · ਕ, ਟ · ਠ · ਡ,
  ਣ · ਜ · ਵ and ੜ · ਘ · ਦ (`tests/filmstrip-target-counts/punjabi.json`:
  29). A list that holds a vowel sign, the halant, the addak or the dot below
  stays undrawn, because those signs have no stroke-order source. A Gurmukhi
  WORD is still never composed, since one headline runs across it.
  `SEPARATE_LETTER_SCRIPTS` is unchanged, and the sequence-strip cases now
  pin a Gurmukhi list (drawn) and the word ਕਰ (refused).
- New `data/scripts/gurmukhi.json` holds exactly the 33 letters, 14 signs and
  4 digits (੦ ੧ ੨ ੫) the Punjabi track reads. Each `sound` comes from the
  letter's own lesson romanization, and `complete` is false. The corpus
  glyph-gap queue stays empty, and Punjabi's letter-anchoring ceilings are
  unchanged (no unread inventory letter). 27 letter rows carry components,
  strokeOrder, strokeOrderNote, penLifts and strokeOrderSource {citation,
  url pinned to commit and line, variation}, cited to GNPS's Gurmukhi Sikho
  tracing lesson with Omniglot copyist shares. The other rows are
  recognition-only.
- New `tests/script-inventories/gurmukhi.evidence.ts` pins the inventory's
  exact glyph set, the 27 cited rows (lifts and source line) and that no sign
  claims an order. The candidate test's switched-off example moves from
  Punjabi to Spanish, since every track with a script of its own is now on.
- Regenerated: 29 Punjabi SVGs, the new Punjabi figure-hash owner, and 18
  Punjabi book chapters (an `\hlblockfigure` line per strip). Narration,
  modality and lesson prose do not change, and no lesson duration moves.
