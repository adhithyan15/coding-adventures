### Fixed Devanagari pen lifts for eleven more letters, and two-line captions

- `अ`, `आ`, `ओ`, `झ`, `स`, `ब`, `च`, `थ`, `भ`, `म` and `व` in
  `src/strokes/devanagari.ts` lifted the pen at every restart of their cited
  Commons diagram (Saurmandal) or animation (Opiaterein, JackPotte): 3, 4,
  5, 3, 3, 3, 2, 2, 2, 2 and 2 times. Only 22%, 25%, 20%, 39%, 28%, 26%,
  17%, 26%, 27%, 20% and 28% of HP Labs India's native writers
  (hpl-dvng-iso-char, counted from the prototypes in the MIT-licensed LipiTk
  4.0 Devanagari recognizer) use those counts.
- They now lift 2, 3, 4, 2, 2, 2, 1, 1, 1, 1 and 1 times, the native modal
  count (74%, 71%, 60%, 53%, 71%, 71%, 76%, 72%, 72%, 80% and 69% of
  writers). Each removes exactly one lift: the run that ends at the right
  (or inner) stem now climbs that stem and descends it, as `औ` already did.
  `अ`, `आ` and `ओ` join the middle shoulder to the stem; `झ` and `स` join
  the crossbar; `ब` joins the oval; `च`, `थ`, `भ`, `म` and `व` join the
  body. The headline is still drawn last, after a lift.
- **The movements stay.** Every source run is still a segment, in the same
  order, from the same start and in the same direction. Each join adds one
  connector that climbs the stem; `ब`'s connector first crosses the oval's
  tail into the stem. The paths are the existing Noto Sans Devanagari fits.
  `fractionOnInk` is 1.0000 on every merged stroke except `ब`'s (0.9967),
  `थ`'s (0.9887) and `व`'s (0.9966), each higher than before; no override
  is added and no ink is left untraced. Every join is exact.
- **Left alone.** `ख` (its upper loop ends where it began, far from the
  stem, so a join would retrace half the loop), `त` (its body ends at the
  open tip, so a join would retrace the whole curve) and `ह` (48% of
  writers use two strokes and 45% use our three: a near tie).
- **Captions.** Every Devanagari headline caption now reads "lift, then
  draw the shirorekha rightward". Every other caption that wrapped past two
  lines at the 23-character frame width was shortened (55 of them), for
  example "descend the left stem, circle clockwise through the loop, and
  sweep right" became "descend, loop clockwise, sweep right". All 44
  Devanagari filmstrips now fit every caption in two lines.
- **Tests.** The eleven shape tests and eleven source tests in
  `tests/strokes/devanagari.test.ts` pin the merged runs, each climb and
  join, and the new variation phrases; `औ`'s test now matches `आ`'s
  joined shoulder run. All 28 Devanagari filmstrips in
  `tests/ductusview/devanagari.test.ts` re-pin their captions, lifts and
  summaries. `stroke-ownership.test.ts` re-pins only `nonTamilDataHash`.
  The filmstrip-geometry ledger was regenerated: the Devanagari ledger now
  averages 1.80 lifts over its 44 glyphs (2.05 before; native modal 1.74).
