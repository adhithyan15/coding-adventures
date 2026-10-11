### Changed Latin: the paths move to a one-storey-a outline, and 13 glyphs join (a d p q t y H á é í ó ú ü)

- The Latin owner's outline is now `LatinPrint-Subset.ttf`, a renamed subset
  of SIL Global's literacy typeface Andika 7.000 (OFL-1.1), whose default a
  is the one-storey a every cited source teaches; Noto Sans's two-storey a
  had held back the letter and about twenty lessons. All 18 earlier paths
  are refitted to it (same order, starts, directions and lifts): every
  stroke 1.000 on ink, nothing untraced, default tolerances, no override.
- New, cited to the Grundschrift-App (facts only) with UJIpenchars2 counts:
  a, d, q (the bowl anticlockwise from the top right, back up the stem, then
  down; UJI writes q mostly in two strokes, recorded), p (stem, back up,
  bowl clockwise), t (stem, then crossbar), y (two strokes; UJI mostly one,
  recorded), H (three strokes; UJI splits on the crossbar's place, recorded).
- New, cited to UJIpenchars2 for the mark, the base letter after the
  Grundschrift-App: á é í ó ú (the letter, a lift, then the acute up to the
  right, the majority's way, about 62% against 30% down-left, recorded) and
  ü (the u, then the left dot, then the right dot: 116 of 118 left first).
  Each precomposed letter's first stroke is exactly its base letter's.
- Still not drawn: grave, circumflex, cedilla, macron, æ, œ (no source);
  ä ö ë ï ÿ (analogy only).
- `latinOutline` reads the new font; `tests/strokes/latin.test.ts` and
  `tests/ductusview/latin.test.ts` cover the 31 glyphs (font-dependent
  positions re-measured on the new outline). stroke-ownership: keys
  622 -> 635, `latin: 31`, key hash and non-Tamil data hash move. The
  filmstrip ledger's Latin owner now holds the 28 glyphs a lesson draws.
