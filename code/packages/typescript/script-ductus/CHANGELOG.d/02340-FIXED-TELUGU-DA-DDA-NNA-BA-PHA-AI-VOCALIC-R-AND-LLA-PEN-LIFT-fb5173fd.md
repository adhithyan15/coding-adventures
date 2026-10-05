### Fixed Telugu da, dda, nna, ba, pha, ai, vocalic r and lla pen lifts; two-line captions

- `ద`, `డ`, `ణ`, `బ`, `ఫ`, `ఐ`, `ఋ` and `ళ` in `src/strokes/telugu.ts`
  lifted the pen after every movement of their tracing source: 4, 4, 4, 3,
  4, 4, 5 and 3 times. HP Labs India's native-writer samples
  (hpl-telugu-iso-char, counted from the prototypes in the MIT-licensed
  LipiTk 4.0 Telugu recognizer) put the modal stroke count at 1, 2, 1, 1,
  3, 1, 3 and 1, and 0% of those writers used our counts.
- Each glyph now has that modal count: 0, 1, 0, 0, 2, 0, 2 and 0 lifts. The
  movements, their order and their directions stay, as segments inside the
  merged runs. Lifts fall only where a part stands apart or the pen sits at
  a dead end: డ's separate top flourish, ఫ's separate top flourish and lower
  stem, and the tips of ఋ's first and middle lobes. ద now draws exactly as
  the body and joined flourish of ధ, and డ as the body and flourish of ఢ.
  Where Noto Sans Telugu joins parts the source drew apart, the run gains
  one segment: బ curves down through the shoulder between its two left
  curves, and ద and ళ climb up the flourish's or chevron's left arm.
- Every centre line was refitted to the bundled Noto Sans Telugu outline.
  `fractionOnInk` is 1.0000 on every stroke except డ's body (0.9975), so the
  డ (0.32), ఐ (0.59) and ఋ (0.84) on-ink overrides and the ణ (0.05), బ
  (0.07) and ళ (0.07) untraced overrides are removed; the default 0.97 floor
  and 2% ceiling apply. Only ణ leaves ink untraced: the tip of Noto's shelf
  inside the left bowl, 10 of 746 sampled points (1.3%). Every join inside a
  run is exact.
- Captions were shortened so that none of the 43 Telugu filmstrips wraps
  past two lines, even at 23 characters a line: 54 captions in 23 other
  glyphs, plus the long ones in ణ, ఫ and ళ. Examples: "loop counterclockwise
  around the left bowl" became "loop the left bowl counterclockwise", and
  "restart at the junction and sweep up through the broad outer arch" became
  "restart and sweep up the broad outer arch".
- The eight tests now pin the grouped runs with every segment label in
  order, and note the native share; the renamed captions are pinned where
  they were before.
- `tests/stroke-ownership.test.ts`: only `nonTamilDataHash` moves, measured
  after the last caption was settled. The filmstrip-geometry ledger was
  regenerated.
