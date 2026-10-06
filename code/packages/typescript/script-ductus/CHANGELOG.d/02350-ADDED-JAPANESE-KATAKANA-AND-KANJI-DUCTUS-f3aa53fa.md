### Added Japanese katakana and kanji ductus

- The katakana `コ` and `ヒ` and the kanji `語`, `日` and `本` enter
  `src/strokes/japanese.ts`: the signs with inventory rows that the chapter
  5 and 6 writing lessons teach (JA-W06-ko-katakana, JA-W06-hi-katakana,
  JA-W05-go-kanji, -nichi-kanji, -hon-kanji). Their rows said only
  "authoritative", so they had no ductus and those five lessons printed no
  filmstrip. Each row now has a `strokeOrderSource`.
- **Order and direction come from KanjiVG**, `kanji/<code point>.svg` for
  each sign: one directed path per stroke, in writing order. `コ` is two
  paths (the top bar turning down the right side, then the base bar); `ヒ`
  two (the short bar, left to right and rising, then the vertical turning
  right along the base); `日` four (left side down, top turning down the
  right side, middle bar, base); `本` five (bar, stem, left sweep, right
  sweep, short lower bar); `語` fourteen: `言`'s seven (top mark, three bars,
  its box), `五`'s four (top bar, a short stroke falling down and to the
  left, the middle bar turning down, the long base bar) and `口`'s three.
- **The coordinates are the print glyph's.** Waypoints read off the bars of
  the Noto Sans JP subset glyph are snapped to the ridge of its distance
  transform and joined by the same shortest path as before. A stroke that
  turns a corner without lifting is two segments with an exact join. `本`'s
  first bar was straightened by hand where the ridge dipped into the
  crossing. KanjiVG's first path of `語` is `言`'s dot, drawn down to the
  right; the print glyph draws a short bar, and the path runs along it from
  left to right. The record says so.
- `言`, `五` and `口`, which chapter 5 also writes on their own
  (JA-W05-gen-component, -five-component, -mouth-component), have no
  inventory row, so they get no ductus here and those three lessons still
  print no filmstrip. They are drawn as the parts of `語`.
- Measured, not asserted: `fractionOnInk` is 1.0000 on every stroke, above
  the 0.97 floor with no override. No ink point is left untraced (コ 601, ヒ
  566, 日 928, 語 1355, 本 998 sampled). The five filmstrips were rendered and
  checked by eye; no caption needs more than two lines. In `語`, frame 10 is
  `五`'s second stroke, and "draw the short stroke down, leaning left" took
  three lines there, so that caption reads "draw a short stroke down and
  left".
- New tests pin each glyph's runs, labels, exact joins and KanjiVG citation;
  the directions the captions claim (every bar rightward and every side
  downward, `コ`'s base ending on its right side, `ヒ`'s bar rising from the
  vertical, `語`'s `五` starting its short stroke on the top bar and falling
  to the left with its base the longest and lowest bar, `語`'s `言` bars each
  lower than the last, `語`'s `言` left of its `五` and `口` and its `五` above
  its `口`, `本`'s sweeps starting at the crossing and falling to either side);
  and that each of the five rows has a cited source whose lift count matches
  its ductus.
- `tests/stroke-ownership.test.ts` was re-measured after the last caption
  was settled: keys 464 -> 469, Japanese 75 -> 80, plus the ordered key hash
  and the non-Tamil data hash. Tamil and both shared-identity values do not
  move. The filmstrip-geometry ledger was regenerated.
