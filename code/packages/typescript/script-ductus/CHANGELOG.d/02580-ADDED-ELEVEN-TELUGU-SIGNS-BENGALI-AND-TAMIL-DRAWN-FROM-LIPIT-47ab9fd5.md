### Added — eleven Telugu signs, Bengali ং and Tamil ஸ, drawn from LipiTk native-writer traces

- **Eleven new Telugu entries** at the end of `src/strokes/telugu.ts`, keyed
  `telugu:<sign>` and sourced through a new `teluguMarkSource` to the mark
  records in `data/scripts/telugu.json`: ం (one ring from its top,
  anticlockwise), ా (the bar from its left end, then the loop clockwise to
  the tip under the bar), ి (from the tail's lower-left tip, anticlockwise,
  curling in), ీ (ి's loop, back along its top, then up over the hook), ు
  (from the lower-left tip, round the bowl, up to the upper tip), ూ (ు, then
  the bar and the loop on the right), ె (from the lower tip, round the right,
  back left along the bar), ే (ె; a lift; the hook from its foot, clockwise),
  ొ (from the foot of the left bowl, over two arches, the loop clockwise), ో
  (ొ, then up into the hook) and ్ (from the lower bar, clockwise through
  both bowls, out along the middle prong and back, out along the top bar).
  ూ reuses ు's first two movements, ో ొ's first three and ే ె's whole first
  stroke, point for point, because Noto prints the same ink there. Each is
  drawn alone, with no consonant, and fitted to the bundled Noto Sans Telugu
  outline of the sign by itself; every stroke is at least 99.7% on ink and no
  ink is left untraced, at the default tolerances.
- **Bengali ং** at the end of `src/strokes/bengali.ts`: the ring
  counterclockwise from its top, a lift, then the tail down to the right.
- **Tamil ஸ** as a new owner, `src/strokes/tamil/U-BB8.ts`, assembled after
  the pulli in `tamil.ts`: one stroke from the tip inside the small left loop,
  over the big arch, down the stem and back up it, over the second arch,
  round the bowl and up the tail.
- **The source.** HP Labs India's LipiTk 4.0 Telugu, Bangla and Tamil
  recognizers (`lipi-reco-indic-char` 4.0.0, MIT model), counted from their
  stored prototypes; counts and shares only, no trace copied. Stroke counts:
  ం 103/104, ా 308/312, ి 204/205, ీ 180/213, ు 405/416, ూ 482/517, ె
  210/210, ే 163/206 two strokes, ొ 294/303, ో 301/320, ్ 101/104, ং 183/189
  two strokes, ஸ 150/153. The weakest claims, recorded as such: the turn of
  ా's and ూ's loop (clockwise 190/308 and 273/482) and ং's order (ring first
  103/183, the commonest form 71/183).
- **Left out.** ై: the recognizer's ai class stores only the length mark
  below (ౖ), never the e hook above it, so the order of its two parts is
  unattested. ృ and ౌ have no class in the recognizer.
- **Evidence.** New `tests/strokes/telugu-marks.test.ts` (hashes, classes,
  lifts, labels, start, turn and end geometry, ై ృ ౌ absent) and
  `tests/ductusview/telugu-marks.test.ts` (frames, lifts, summaries); new
  `tests/strokes/tamil/U-BB8.test.ts` and `tests/ductusview/tamil/U-BB8.test.ts`;
  `tests/strokes/bengali.test.ts` and `tests/ductusview/bengali.test.ts` take
  ং (ten glyphs).
- **Pins and ledger.** `tests/stroke-ownership/`: telugu 43 -> 54, bengali
  9 -> 10, tamil 36 -> 37 (tail run 10 -> 11); key and data hashes move; no
  other script changes. `filmstrip-geometry.d/` gains the thirteen glyphs for
  telugu, bengali and tamil; no existing entry changes.
