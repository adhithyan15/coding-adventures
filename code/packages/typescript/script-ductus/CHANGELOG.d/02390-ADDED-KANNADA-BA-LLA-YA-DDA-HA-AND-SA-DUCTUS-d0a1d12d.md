### Added Kannada ba, lla, ya, dda, ha and sa ductus

- `ಬ` (U+0CAC), `ಳ` (U+0CB3), `ಯ` (U+0CAF), `ಡ` (U+0CA1), `ಹ` (U+0CB9) and
  `ಸ` (U+0CB8) enter `src/strokes/kannada.ts`. Their bare-consonant rows in
  `kannada.json` (role `syllable`) now carry a `strokeOrderSource`.
- **Order and direction come from Gopala Krishna A's Commons animations**,
  read frame by frame from a mirror copy of each GIF whose pixel size and
  byte size match the Commons category listing (Commons is not reachable
  from the authoring sandbox): `Kannada-alphabet-ba.gif` (29 frames),
  `-lla.gif` (35), `-ya.gif` (35), `-da.gif` (49), `-ha.gif` (40) and
  `-sa.gif` (40), all at 10 fps. The frames show that "da" draws retroflex
  ಡ, as the first batch found; dental ದ is "dha".
- **Runs and lifts.** ಬ is one run with no lift: from the curled tip inside
  the head, over the head, round the left lobe into the middle point, round
  the right lobe and up the tall right side. ಳ closes its small loop, sweeps
  round the outer left side, circles the lower loop and climbs the right
  bowl; ಡ runs through the left lobe, the middle point, the right lobe and
  its small inner loop and back along the top; ಹ closes the left ring, arches
  into the right ring, rounds it and rises up the neck. Each of those three
  then lifts once for the top bar and hook. ಸ takes three runs (two lifts):
  the body from its tail, the hooked bar, then the dot. ಯ takes four (three
  lifts): the round bowl, a fresh stroke from its foot up the middle arm, the
  hooked bar, and the small right bowl from its foot.
- **The coordinates are the print glyph's.** Each path follows the skeleton
  of the bundled Noto Sans Kannada outline between turning points chosen from
  the animation, and every join is exact. Where Noto's shape differs, the
  record says so: ಬ's curled tip is a short wedge; ಳ's small loop shares its
  left side with the outer curve; ಡ's top arc is fused into the bar and its
  inner loop shares its right side with the rise; ಹ's two rings share one
  upright, which the path runs down once for each ring; ಯ's middle arm and
  ಳ's right bowl run into the bar; ಸ's dot is a filled disc, drawn as a small
  closed loop inside it.
- Measured with the default tolerances and no override: `fractionOnInk` is
  1.0000 on every stroke, every join gap is 0, and no ink point is left
  untraced (653 sampled for ಬ, 698 for ಳ, 1170 for ಯ, 856 for ಡ, 889 for ಹ,
  686 for ಸ). The six filmstrips were rendered and checked by eye; no caption
  needs more than two lines.
- New tests pin each letter's runs, lifts, labels and Commons URL, and each
  filmstrip's steps and summary. `tests/stroke-ownership.test.ts` was
  re-measured after the last caption was settled: keys 445 -> 451, Kannada
  19 -> 25, plus the ordered key hash and the non-Tamil data hash. The
  filmstrip-geometry ledger was regenerated.
