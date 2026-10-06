### Added Japanese ni, ha, ma, ri and n ductus

- `に` (U+306B), `は` (U+306F), `ま` (U+307E), `り` (U+308A) and `ん`
  (U+3093) enter `src/strokes/japanese.ts`. Chapters 1 to 4 have long taught
  them, but their inventory rows cited no stroke-order source, so they had no
  ductus and their writing lessons printed no filmstrip. Each row now has a
  `strokeOrderSource`. With them, every one of the 46 basic hiragana, あ to ん
  with を, has a cited stroke order and a ductus, and a new test says so.
- **Order and direction come from KanjiVG**, as for こ to と:
  `kanji/0306b.svg` (three paths), `0306f.svg` (three), `0307e.svg` (three),
  `0308a.svg` (two) and `03093.svg` (one). So に, は and ま have two pen lifts
  each, り one and ん none.
- **The coordinates are the print glyph's.** Each segment runs along the
  Noto Sans JP subset outline's medial line between turning points read off
  that outline. Every sharp turn inside a stroke is a segment boundary, and
  every join is exact. Three strokes reuse ink on purpose. The flick of に's
  and は's left vertical leaves the print stroke above its foot, so, as in け,
  the path goes down to the foot and climbs back to it. ん's hump leaves the
  diagonal partway up, so, as in え, the path goes down to the tip and climbs
  back up that ink. り's print glyph has no flick at the foot of its left
  stroke; a thin rise leaves that stroke partway down and arches into the top
  of the right one. The path climbs back to that rise and follows it to the
  top of the arch, where the right stroke begins, so in the print glyph the
  two strokes touch, though KanjiVG's do not. The record says so.
- Measured, not asserted: `fractionOnInk` is 1.0000 on every stroke, above
  the 0.97 floor with no override. No ink point is left untraced (592
  sampled for に, 861 for は, 822 for ま, 587 for り, 592 for ん). The five
  filmstrips were rendered and checked by eye; no caption needs more than
  two lines.
- New tests pin each letter's run and lift counts, its labels in order,
  exact joins, and its KanjiVG citation and variation phrases. One more test
  checks the directions the captions claim. に's and は's left vertical runs
  down, climbs back up its own ink and flicks up to the right. Every bar runs
  rightward. に's lower stroke curves down and runs out to the right. は's and
  ま's last stroke starts above the bars, runs down through them, loops left
  and back up, crosses the vertical and runs out to the lower right. り's left
  stroke climbs back up its own ink and rises over the arch; the right stroke
  begins where that rise ends, comes down and sweeps to the lower left. ん
  cuts down to the lower left, climbs back up the same ink, turns over the
  hump and rises to the right at the end. A third test checks that all 46
  basic hiragana have a cited source in the inventory and a ductus.
- `tests/stroke-ownership.test.ts` was re-measured after the last caption
  was settled: keys 434 -> 439, Japanese 45 -> 50, plus the ordered key hash
  and the non-Tamil data hash. Tamil and both shared-identity values do not
  move. The filmstrip-geometry ledger was regenerated.
