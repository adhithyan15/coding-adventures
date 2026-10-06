### Fixed Devanagari pen lifts for eleven letters

- `क`, `य`, `र`, `प`, `ध`, `ल`, `द`, `ठ`, `घ`, `ष` and `औ` in
  `src/strokes/devanagari.ts` lifted the pen at every restart of their cited
  Commons animation (Opiaterein) or panel diagram (Saurmandal, `औ`): 3, 3,
  2, 2, 3, 3, 2, 2, 2, 3 and 6 times. HP Labs India's native-writer samples
  (hpl-dvng-iso-char, counted from the prototypes in the MIT-licensed LipiTk
  4.0 Devanagari recognizer) show only 1%, 0%, 1%, 2%, 3%, 4%, 5%, 11%, 12%,
  12% and 15% of writers using those counts.
- They now lift 1, 1, 1, 1, 1, 1, 1, 1, 1, 2 and 5 times, the native modal
  count (74%, 89%, 96%, 98%, 81%, 54%, 95%, 89%, 83%, 83% and 60% of
  writers). The headline is still drawn last, after a lift; `ष` also lifts
  for its inner diagonal, and `औ` keeps every restart except the one
  between the middle shoulder and the inner stem.
- **The movements stay.** Every animated run is now a segment, in the same
  order, from the same start and in the same direction. Where the next run
  starts somewhere else, the stroke gains a connecting segment: `क`, `य`,
  `प`, `ध`, `ल` and `औ` climb the stem they are about to descend, `क` then
  climbs back to the arch's junction, `ध` turns back along its shoulder to
  the waist where its bowl begins, and `घ` comes back down its right side
  into the short stem. `य`'s bowl and `र`'s tail now start exactly where
  the curl and loop end (5 and 14 font units from their old starts).
- **Fit.** The paths are the existing Noto Sans Devanagari fits plus the
  connectors. `fractionOnInk` is 1.0000 on every stroke except `क`'s body
  (0.9934), `द`'s body (0.9963) and the unchanged `ष` diagonal (0.9907) and
  `औ` lower arc (0.9872); no override is added and the default 0.97 floor
  and 2% untraced ceiling apply. No ink point is left untraced (732, 541,
  368, 495, 575, 639, 516, 555, 573, 595 and 1092 sampled). Every join
  inside a run is exact.
- **Captions.** All eleven filmstrips were rendered and every caption fits
  in two lines. The shared headline caption becomes "lift, then draw the
  shirorekha rightward" in these eleven, because "shirorekha left-to-right"
  cannot share a line, and the longer body captions were shortened.
- **Tests.** The eleven shape tests in `tests/strokes/devanagari.test.ts`
  now pin the merged runs, each segment's direction and every join; the
  eleven source tests pin the new variation phrases. The eight
  `tests/ductusview/devanagari.test.ts` filmstrips pin every caption, lift
  and summary. `stroke-ownership.test.ts` re-pins only `nonTamilDataHash`.
  The filmstrip-geometry ledger was regenerated: the Devanagari ledger now
  averages 2.05 lifts over its 44 glyphs (2.39 before; native modal 1.74).
