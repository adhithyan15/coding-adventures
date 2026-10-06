### Added Gurmukhi: a ductus owner for 27 letters cited to GNPS's tracing lesson

- New owner `src/strokes/gurmukhi.ts` (keys `gurmukhi:<glyph>`, appended
  after Bengali so no key moves): the vowel bearer ਅ and 26 consonants, ਸ ਹ
  ਕ ਖ ਗ ਘ ਚ ਛ ਜ ਟ ਠ ਡ ਣ ਤ ਥ ਦ ਨ ਪ ਫ ਬ ਭ ਮ ਰ ਲ ਵ ੜ. These are every letter a
  Punjabi writing lesson prints alone or in a list of letters.
- The order, start, direction and lifts come from the Alphabet Tracing
  lesson of GNPS's Gurmukhi Sikho app (`codemanxdev/gnps_learning_hub`,
  Apache-2.0, `lib/data/lessons/lesson_tracing.dart` at commit `de1e560`, one
  line per letter). Its checkpoints were entered by the developer over Noto
  Sans Gurmukhi, one list per pen-down stroke, and the app makes a child reach
  them in order. No checkpoint coordinate is copied: every path follows the
  skeleton of the bundled Noto Sans Gurmukhi outline. Default tolerances, no
  overrides: every stroke is 100% on ink, every join is exact and nothing is
  left untraced. The source of every path is read from the letter's own row
  in `data/scripts/gurmukhi.json`, so the two cannot drift.
- The headline is drawn first, left to right, as the source teaches it; each
  record says this is a teaching order and that fluent writers are often
  described as adding it last. Noto prints a split headline in ਅ ਖ ਘ ਪ ਮ.
  The source draws no separate bar for these letters, and neither do the
  paths: the bar's left part opens the first stroke, and its right part,
  drawn right to left, opens the stem stroke.
- Where Noto prints a loop as a solid knob or tail (ਅ ਸ ਚ ਜ ਡ ਤ ਦ ਮ ੜ) or a
  solid upright (ਘ), the path loops inside it. Each loop turns the same way
  as the source's stroke.
- ਛ, ਨ and ਬ lift once less than the source. Omniglot's non-native copyists
  (Lake et al. 2015, MIT) most often use one stroke fewer, and copyist counts
  are an upper bound on native lifts. ਛ runs back along its bowl's top to the
  middle stroke. ਨ's left leg continues from the stem, where it starts anyway.
  ਬ runs back along its bar to the lower bowl.
- `src/scriptdata.ts` adds the Gurmukhi inventory as the last script, and
  `tests/support/font-fixtures.ts` gains `gurmukhiOutline`. New tests:
  `tests/strokes/gurmukhi.test.ts` checks honesty at the default tolerances,
  the lifts, headline first or split, the turning sense, the joins and the
  source. `tests/ductusview/gurmukhi.test.ts` checks the frames, lifts and
  summaries.
- `tests/stroke-ownership.test.ts`: keys go 534 -> 561 with a new
  `gurmukhi: 27` count, and the ordered key hash and the non-Tamil data hash
  move (commented). `gurmukhi` joins the owner-module list. The
  filmstrip-geometry ledger gains the owner `gurmukhi` (27 entries). No other
  owner changes.
