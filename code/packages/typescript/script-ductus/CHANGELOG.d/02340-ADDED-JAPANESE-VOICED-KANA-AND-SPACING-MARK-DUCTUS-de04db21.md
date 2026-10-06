### Added Japanese voiced kana and spacing mark ductus

- The 22 voiced kana the Japanese inventory holds, `が` to `ぽ` (が ぎ ぐ げ ご
  ざ ず ぜ ぞ だ で ど ば び ぶ べ ぼ ぱ ぴ ぷ ぺ ぽ), and the three spacing marks
  `゛` (U+309B), `゜` (U+309C) and `ー` (U+30FC) enter
  `src/strokes/japanese.ts`. Their rows cited no stroke-order source (だ and ど
  had no row), so they had no ductus, and twenty writing lessons printed no
  filmstrip. Each row and mark record now has a `strokeOrderSource`.
- `strokeSource` in `japanese.ts` now looks a glyph up in the inventory's
  `marks` as well as its `letters`, the way `kannada.ts` does, and throws if
  neither holds a cited source. The provenance gate in `tests/strokes.test.ts`
  and `fontForDuctus` already accepted a mark's claim.
- **Order and direction come from KanjiVG.** Each voiced kana is one file,
  `kanji/<code point>.svg`, whose first paths are the base sign's own (the
  same as in its file for the base sign) and whose last are the mark: the
  dakuten's two ticks, the left one first, each down to the right
  (`0309b.svg`), or the handakuten's one ring, from its foot, clockwise
  (`0309c.svg`). `ー` is one bar, left to right (`030fc.svg`). So a dakuten
  adds two pen lifts to the base sign and a handakuten one.
- **The coordinates are the print glyph's.** Each base sign's fitted path is
  moved by the offset measured between the two glyphs' outlines (the Noto
  Sans JP subset draws the base of a voiced kana a few units left and down
  of the plain sign) and re-fitted to the voiced glyph's medial line with the
  same distance-transform ridge and shortest path as before. The ticks' ends
  and the ring's points are read off the mark contours and held fixed, so
  that a short blob's ridge maximum does not pull a tick's two ends together
  or let the ring's start drift away from its foot. Every join is exact.
- **Where KanjiVG disagrees with a base row that cites Sirgazil, the voiced
  glyph follows KanjiVG**, and its record says so: ぜ draws the right stem
  second and the left stem third (the せ row has the reverse); ぶ and ぷ draw
  the lower-left mark up to the right (the ふ row draws it down to the left);
  ぼ and ぽ end the left vertical in a flick up to the right, as は does (the
  ほ row says it hooks left). The せ, ふ and ほ entries are not changed.
- Voiced glyphs whose base sign cites Sirgazil (ぐ, だ, で, び, ぶ, ぴ, ぷ, ぜ)
  use shorter captions than the base sign, so that no frame needs three
  lines; the base signs keep theirs.
- Measured, not asserted: `fractionOnInk` is 1.0000 on every stroke, above
  the 0.97 floor with no override. No ink point is left untraced (が 813, ぎ
  823, ぐ 462, げ 740, ご 514, ざ 717, ず 721, ぜ 824, ぞ 711, だ 727, で 584,
  ど 602, ば 923, び 780, ぶ 629, べ 497, ぼ 980, ぱ 955, ぴ 811, ぷ 659, ぺ 526,
  ぽ 1013, ゛ 89, ゜ 98, ー 254 sampled). The twenty filmstrips were rendered
  and checked by eye; no caption needs more than two lines. The marks'
  citations name the code point without the long upper-case Unicode name,
  which overflowed the footnote of a one- or two-frame strip.
- New tests pin each glyph's run and lift counts, its labels in order, exact
  joins, and its KanjiVG citation and variation phrases. One more test checks
  the directions the captions claim: every dakuten is the last two strokes,
  left tick first, each running down to the right, at the right of the base
  sign and in its upper half; every handakuten is one closed ring, last, that
  starts at its foot and swings left first; ー runs left to right; ぜ's right
  stem comes before its left; ぶ's and ぷ's lower-left mark runs up to the
  right; ぼ's and ぽ's left vertical flicks up to the right and their last
  base stroke starts at the upper bar, loops left and runs out to the lower
  right. A third test checks that every voiced hiragana row and every mark
  record has a cited source and a ductus.
- `tests/stroke-ownership.test.ts` was re-measured after the last caption
  was settled: keys 439 -> 464, Japanese 50 -> 75, plus the ordered key hash
  and the non-Tamil data hash. Tamil and both shared-identity values do not
  move. The filmstrip-geometry ledger was regenerated.
