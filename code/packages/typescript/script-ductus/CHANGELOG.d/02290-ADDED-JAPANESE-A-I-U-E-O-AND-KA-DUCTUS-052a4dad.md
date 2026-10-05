### Added Japanese a, i, u, e, o and ka ductus

- `あ` (U+3042), `い` (U+3044), `う` (U+3046), `え` (U+3048), `お`
  (U+304A) and `か` (U+304B) enter `src/strokes/japanese.ts`. Chapters 1,
  3 and 10 have long taught them, but their inventory rows cited no
  stroke-order source, so they had no ductus and their writing lessons printed
  no filmstrip. Each row now has a `strokeOrderSource`. Without a ductus, the
  gate that requires every verified claim to be drawable would fail.
- **Order and direction come from KanjiVG**, as for き, け, ぬ, へ and ら:
  `kanji/03042.svg` (three paths), `03044.svg` (two), `03046.svg`
  (two), `03048.svg` (two), `0304a.svg` (three) and `0304b.svg` (three).
  So あ, お and か have two pen lifts each, and い, う and え one each.
- **The coordinates are the print glyph's.** Each segment runs along the
  Noto Sans JP subset outline's medial line between turning points read off
  that outline. Every sharp turn inside a stroke is a segment boundary, and
  every join is exact. え's hump branches off its diagonal in the print
  glyph. KanjiVG's handwriting climbs back up beside the diagonal, so the
  path goes down to the tip of the diagonal and retraces its ink up to the
  branch before turning over the hump.
- Measured, not asserted: `fractionOnInk` is 1.0000 on every stroke, above
  the 0.97 floor with no override. No ink point is left untraced (969 sampled
  for あ, 496 for い, 542 for う, 663 for え, 861 for お, 737 for か). The six
  filmstrips were rendered and checked by eye. か's last caption first ran to
  three lines, so the last stroke of お and か is now "draw the dot down to
  the right", and no caption needs more than two lines.
- New tests pin each letter's run and lift counts, its labels in order,
  exact joins, and its KanjiVG citation and variation phrases. One more test
  checks the directions the captions claim. あ's third stroke cuts down to the
  lower left, loops back across the vertical and finishes low, heading left.
  い flicks up to the right. う rises along the top and finishes at the lower
  left. え's climb stays on the diagonal's own ink. お loops left of its
  vertical before rounding the bowl. か's bar turns down and hooks back left.
- `tests/stroke-ownership.test.ts` was re-measured: keys 423 -> 429,
  Japanese 34 -> 40, plus the ordered key hash and the non-Tamil data hash.
  Tamil and both shared-identity values do not move. The filmstrip-geometry
  ledger was regenerated.
