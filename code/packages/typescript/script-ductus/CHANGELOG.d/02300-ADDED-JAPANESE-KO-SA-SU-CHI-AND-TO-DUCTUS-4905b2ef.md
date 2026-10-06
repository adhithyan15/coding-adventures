### Added Japanese ko, sa, su, chi and to ductus

- `こ` (U+3053), `さ` (U+3055), `す` (U+3059), `ち` (U+3061) and `と`
  (U+3068) enter `src/strokes/japanese.ts`. Chapters 2 to 4 have long taught
  them, but their inventory rows cited no stroke-order source, so they had no
  ductus and their writing lessons printed no filmstrip. Each row now has a
  `strokeOrderSource`. Without a ductus, the gate that requires every
  verified claim to be drawable would fail.
- **Order and direction come from KanjiVG**, as for あ to か:
  `kanji/03053.svg` (two paths), `03055.svg` (three), `03059.svg` (two),
  `03061.svg` (two) and `03068.svg` (two). So さ has two pen lifts, and こ,
  す, ち and と one each.
- **The coordinates are the print glyph's.** Each segment runs along the
  Noto Sans JP subset outline's medial line between turning points read off
  that outline. Every sharp turn inside a stroke is a segment boundary, and
  every join is exact. Two strokes reuse ink on purpose, as え's does. す's
  vertical runs down the right side of the loop, and the loop comes back
  down that same band into the tail, as KanjiVG's path does. ち's descender
  stops at its tip and turns back up the same ink before rising to the
  right. と's short stroke ends on the long sweep, because the print glyph
  joins them there and KanjiVG's first path ends on its second.
- Measured, not asserted: `fractionOnInk` is 1.0000 on every stroke, above
  the 0.97 floor with no override. No ink point is left untraced (438
  sampled for こ, 642 for さ, 655 for す, 708 for ち, 541 for と). The five
  filmstrips were rendered and checked by eye; no caption needs more than
  two lines.
- New tests pin each letter's run and lift counts, its labels in order,
  exact joins, and its KanjiVG citation and variation phrases. One more test
  checks the directions the captions claim. こ's lower stroke and さ's foot
  curve down and run out to the right. さ's second stroke starts above the
  bar, slants down to the right and hooks back left. す's loop swings left
  and over the top, and its tail starts on the vertical's own ink. ち falls
  to the lower left, turns back up its own ink and finishes low, heading
  left. と's short stroke ends on the long sweep.
- `tests/stroke-ownership.test.ts` was re-measured after the last caption
  was settled: keys 429 -> 434, Japanese 40 -> 45, plus the ordered key hash
  and the non-Tamil data hash. Tamil and both shared-identity values do not
  move. The filmstrip-geometry ledger was regenerated.
