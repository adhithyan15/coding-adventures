### Added Japanese ki, ke, nu and he ductus

- `き` (U+304D), `け` (U+3051), `ぬ` (U+306C) and `へ` (U+3078) enter
  `src/strokes/japanese.ts` for the chapter 133 writing lessons, which
  write the last four signs of the basic hiragana table. Each one already has
  a `strokeOrderSource` in `data/scripts/japanese.d`. Without a ductus, the
  gate that requires every verified claim to be drawable would fail.
- `ら` (U+3089) enters too. Chapter 8 has written it since JA-W08-ra, but it
  had no inventory row, and so no source and no ductus. Its new record cites
  `kanji/03089.svg` (two paths, one lift), and its ductus is fitted the same
  way as the other four.
- **Order and direction come from KanjiVG**, as for `を`, `そ`, `れ` and `る`:
  `kanji/0304d.svg` (four paths), `kanji/03051.svg` (three), `kanji/0306c.svg`
  (two) and `kanji/03078.svg` (one). So き has three pen lifts, け two, ぬ one
  and へ none.
- **The coordinates are the print glyph's.** Each segment runs along the
  Noto Sans JP subset outline's medial line between turning points read off
  that outline. The ridge is a distance-transform maximum, and the path
  between turning points is a shortest path that prefers it. Every sharp turn
  inside a stroke is a segment boundary, and every join is exact. Where the
  print glyph has no separate ink for a return, the path retraces the ink:
  け goes down to the foot of its left stroke and climbs back a short way
  into the flick.
- Measured, not asserted: `fractionOnInk` is 1.0000 on every stroke, above
  the 0.97 floor with no override. No ink point is left untraced (753 sampled
  for き, 667 for け, 1010 for ぬ, 415 for へ, 626 for ら). The five
  filmstrips were rendered and checked by eye, and no caption needs more than
  two lines.
- New tests pin each letter's run and lift counts, its labels in order,
  exact joins, and its KanjiVG citation and variation phrases. One more test
  checks the directions the captions claim: き's diagonal runs down to the
  right and hooks back left, け's left stroke flicks up to the right and its
  long stroke ends to the left, ぬ's last part swings back round the small
  loop before flicking out, へ's peak is its highest point, left of the
  middle, and ら's bowl rises right of its left stroke and finishes low on
  the left.
- `tests/stroke-ownership.test.ts` was re-measured: keys 418 -> 423,
  Japanese 29 -> 34, plus the ordered key hash and the non-Tamil data hash.
  Tamil and both shared-identity values do not move. The filmstrip-geometry
  ledger was regenerated.
