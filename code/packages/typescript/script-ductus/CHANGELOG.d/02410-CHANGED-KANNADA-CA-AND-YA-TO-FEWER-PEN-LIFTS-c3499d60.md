### Changed Kannada ca and ya to fewer pen lifts

- `ಚ` (U+0C9A) goes from four strokes and three lifts to two strokes and one
  lift. Its first stroke keeps the body's four segments, then gains "run back
  left along the lower bar" ((636,345) to (447,345)), keeps "draw the lower bar
  rightward", gains "come back to the link's foot" ((755,345) to (636,352))
  and keeps "draw the short link upward". The hooked upper bar is the second
  stroke.
- `ಯ` (U+0CAF) goes from four strokes and three lifts to three strokes and
  two lifts. The middle arm's stroke gains "run back left along the top bar"
  ((752,511) to (632,518)) and continues with the top bar and hook. The bowl
  and the small right bowl stay separate runs: the bowl ends at its top left,
  about 750 font units of ink from the arm's foot, and the hook's tip about
  820 from the right bowl's foot, so joining either would retrace most of a
  curve.
- Why: the Gopala Krishna A animations restart for each part, which shows
  order and direction, not how often writers lift. No native Kannada pen data
  was reachable. Omniglot (Lake, Salakhutdinov & Tenenbaum, Science 2015; MIT
  licence) has 20 drawings per letter by Amazon Mechanical Turk copyists, not
  native writers; against HP Labs India's native data they over-count strokes
  (Devanagari by 0.65, Bangla by 1.21 on the per-glyph mode), so their counts
  are used only as a ceiling. Even so, their mode is two strokes for ಚ (55%)
  and three for ಯ (45%, with two at 40%); four strokes, the old count, is 10%
  for each.
- Every animated movement stays a segment, in its order and direction; the
  connectors run along bar ink the pen is about to draw or has drawn.
  Measured with the default tolerances and no override: `fractionOnInk` is
  1.0000 on every stroke, every join gap is 0, and no ink point is left
  untraced (851 sampled for ಚ, 1170 for ಯ). Both filmstrips were rendered and
  checked by eye; every caption fits in two lines.
- `tests/strokes/kannada.test.ts` pins the merged runs, the connectors' end
  points and the bars' directions; `tests/ductusview/kannada.test.ts` pins
  each filmstrip's steps and summary ("2 strokes · 1 pen lift · 10
  movements"; "3 strokes · 2 pen lifts · 8 movements").
  `tests/stroke-ownership.test.ts` re-pins only the non-Tamil data hash. The
  filmstrip-geometry ledger was regenerated.
