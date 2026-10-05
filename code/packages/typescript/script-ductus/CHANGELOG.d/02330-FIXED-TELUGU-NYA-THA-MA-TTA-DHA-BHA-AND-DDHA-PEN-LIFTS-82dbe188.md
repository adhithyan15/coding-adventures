### Fixed Telugu nya, tha, ma, tta, dha, bha and ddha pen lifts

- `ఞ`, `థ`, `మ`, `ట`, `ధ`, `భ` and `ఢ` in `src/strokes/telugu.ts` lifted
  the pen far more often than native writers do: 7, 6, 6, 5, 5, 5 and 5
  times. Their source, "Sathish Shanmugam, Write Telugu Alphabets", numbers
  directional movements, and each numbered movement had become its own
  pen-down run. HP Labs India's native-writer samples (hpl-telugu-iso-char,
  counted from the prototypes in the MIT-licensed LipiTk 4.0 Telugu
  recognizer) put the modal stroke count at 3, 3, 2, 2, 2, 2 and 3. 0% of
  those writers used our counts.
- Each glyph now has that modal count: 2, 2, 1, 1, 1, 1 and 2 lifts. The
  movements, their start points and their directions stay, as segments inside
  the merged runs. Lifts fall only between the body and a detached part:
  ఞ's right bar and upper stem, థ's lower stem and inner dot, మ's separate
  right bowl, ట's upper stem, ధ's and భ's lower stem, and ఢ's separate top
  flourish and lower stem. Where Noto Sans Telugu joins the top flourish to
  the body (థ, ధ, భ, మ), the run climbs up the flourish's left arm and
  curls back through it, so the flourish joins the body without a lift.
- Every centre line was refitted to the bundled Noto Sans Telugu outline.
  `fractionOnInk` is 1.0000 on every stroke except ట's and ఢ's bodies
  (0.9975 each), all above the 0.97 floor. The ఞ (0.86), ట (0.55) and ఢ
  (0.32) overrides are removed. No ink point is left untraced (798, 750,
  914, 664, 711, 822 and 763 sampled), and every join inside a run is exact.
- Captions were shortened so that none wraps past two lines. Examples:
  "sweep right and upward around the lower-right bowl" became "sweep right
  and up the lower-right bowl", and "curve left around the upper-right
  shoulder" became "curve left round the upper-right shoulder". All seven
  filmstrips were rendered and checked by eye.
- The seven tests now pin the grouped runs, with every segment label in
  order. Each test notes the native share.
- `tests/stroke-ownership.test.ts`: only `nonTamilDataHash` moves, measured
  after the last caption was settled. The filmstrip-geometry ledger was
  regenerated.
