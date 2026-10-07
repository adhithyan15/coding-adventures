### Added Japanese ductus for the chapter 5 components 言, 五 and 口

- The kanji `言`, `五` and `口` enter `src/strokes/japanese.ts`. Chapter 5
  writes each on its own before assembling `語` (JA-W05-gen-component,
  -five-component, -mouth-component), but they had no inventory row, so
  they had no ductus and those three lessons printed no filmstrip. Each is
  now read in a word headword (`言う`, `五`, `口`), so each has a row with a
  `strokeOrderSource`, and all three lessons print a filmstrip.
- **Order and direction come from KanjiVG**, `kanji/08a00.svg`,
  `04e94.svg` and `053e3.svg`: `言` is seven paths (top mark, long bar, two
  short bars, then its box: left side, top turning down the right side,
  base); `五` four (top bar, a stroke falling from it down and to the left
  as far as the base, the middle bar turning down, the long base bar); `口`
  three (left side, top turning down the right side, base). These are the
  same order and directions as `語`'s own `言`, `五` and `口`.
- **The coordinates are the standalone print glyph's.** Waypoints on the
  bars of each Noto Sans JP subset glyph are snapped to the ridge of its
  distance transform and joined by the same shortest path as for `語`; they
  are not `語`'s component paths moved. A stroke that turns a corner without
  lifting is two segments with an exact join. KanjiVG's first path of `言`
  is a dot falling to the right; the print glyph draws a short bar there,
  and the path runs along it from left to right.
- **Measured, not overridden.** fractionOnInk is 1.0000 on every stroke at
  the default floor, with no per-glyph override, and no ink point is left
  untraced (1037, 940 and 865 sampled). The three filmstrips were rendered
  and checked by eye; no caption needs more than two lines. Standalone,
  `五`'s falling stroke runs from the top bar to the base, so its caption is
  "draw the stroke down and left" rather than calling it short.
- Tests: each glyph's runs, labels, joins and citation; every bar rightward
  and every side downward; `五`'s falling stroke starting on its top bar,
  falling left and ending on the base, its base the longest and lowest;
  `言`'s bars each lower than the last; and the three rows' lift counts
  matching their ductus. `tests/stroke-ownership.test.ts` moves keys 534 ->
  537 and Japanese 80 -> 83, with the ordered key hash and the non-Tamil data
  hash; Tamil and the shared-identity values are unchanged. The filmstrip
  geometry ledger is regenerated.
