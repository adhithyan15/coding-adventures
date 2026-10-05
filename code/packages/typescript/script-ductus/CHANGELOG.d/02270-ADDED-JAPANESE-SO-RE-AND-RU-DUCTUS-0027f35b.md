### Added Japanese そ, れ and る ductus

- `そ` (U+305D), `れ` (U+308C) and `る` (U+308B) enter
  `src/strokes/japanese.ts` for the chapter 132 writing lessons. Each one
  already has a `strokeOrderSource` in `data/scripts/japanese.d`. Without a
  ductus, the gate that requires every verified claim to be drawable would
  fail.
- **Order and direction come from KanjiVG**, as for `を`: `kanji/0305d.svg`
  (one path), `kanji/0308c.svg` (two) and `kanji/0308b.svg` (one). So そ and る
  have no pen lift, and れ has one.
- **The coordinates are the print glyph's.** Each segment runs along the
  Noto Sans JP subset outline's medial line between turning points read off
  that outline. The ridge is a distance-transform maximum, and the path
  between turning points is a shortest path that prefers it. Every sharp turn
  inside a stroke is a segment boundary, and every join is exact. Where the
  print glyph has no separate ink for a return, the path retraces the ink.
  そ doubles back along its middle bar into the curve, and れ climbs back up
  its diagonal into the arch.
- Measured, not asserted: `fractionOnInk` is 1.0000 on every stroke, above
  the 0.97 floor with no override. No ink point is left untraced (646 sampled
  for そ, 765 for れ, 766 for る). The three filmstrips were rendered and
  checked by eye. Captions were shortened until each frame needed at most two
  lines.
- New tests pin each letter's run and lift counts, its labels in order,
  exact joins, and its KanjiVG citation and variation phrases. One more test
  checks the directions the captions claim: the top bars run rightward and
  turn down-left, そ swings back right along its bar and finishes on the base,
  る's loop closes back inside the bowl, and れ's flick rises at the far right.
- `tests/stroke-ownership.test.ts` was re-measured: keys 415 -> 418,
  Japanese 26 -> 29, plus the ordered key hash and the non-Tamil data hash.
  Tamil and both shared-identity values do not move. The filmstrip-geometry
  ledger was regenerated.
