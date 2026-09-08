### Added source-verified Japanese ろ and ゅ ductus (HL-C360)

- `ろ` (U+308D) and `ゅ` (U+3085) enter `src/strokes/japanese.ts`, the two
  hiragana the Japanese cardinals one to ten needed. Both already carried a
  `strokeOrderSource` in `data/scripts/japanese.d`, so the corpus gate that
  every verified prose claim has a matching font-checked ductus was red until
  they landed; neither claim was weakened to close it.
- **`ろ` is one pen-down run with zero lifts, and the path is the bundled Noto
  Sans JP subset's own medial line** — the glyph was rasterised, thinned, and
  the skeleton walked from the upper-left origin to the tail, so the shoulder,
  the diagonal and the belly are the font's geometry rather than a second
  drawing. The order comes from the cited animation, read frame by frame: the
  start marker never leaves the upper-left origin across all 26 frames.
- **The pen reaches the FOOT of the diagonal before the belly departs, and that
  is a measurement rather than a preference.** A path that turns where the belly
  leaves instead — the tidier movement — leaves **67 of the letter's 673 sampled
  ink points, 9.96%, more than 100 units from any stroke**, which the coverage
  check rejects at 2%. The letter's own outline says the diagonal is written to
  its end.
- **`ゅ` is two runs with one lift, and claims no independent handwriting
  evidence.** Its order and its five captions are `ゆ`'s, verbatim; the
  coordinates are `ゆ`'s verified path mapped through the two glyphs' bounding
  boxes and then snapped to the SMALL glyph's own medial line, no point moving
  more than 33 units to land on it. That is the rule U+3063 small tsu already
  uses from つ, and the letter's `strokeOrderSource.variation` says so on the
  record.
- Both are checkable rather than asserted: `fractionOnInk` is **1.0000** for
  every stroke of both letters, every segment join is exact, and **zero** ink
  points are left untraced (673 sampled for ろ, 666 for ゅ). The filmstrips were
  rendered and looked at, not merely measured.
- `tests/stroke-ownership.test.ts` pins move because of these two glyphs and
  were re-measured rather than reasoned about: 359 → 361 keys, japanese 15 → 17,
  the ordered key hash and the non-Tamil data hash. Tamil is untouched, so its
  count and both shared-identity values are unchanged.
