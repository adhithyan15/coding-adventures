### Added Kannada ttha ductus

- `ಠ` (U+0CA0) enters `src/strokes/kannada.ts`. Its bare-consonant row in
  `kannada.json` (role `syllable`) now carries a `strokeOrderSource`.
- **Order and direction come from Gopala Krishna A's Commons animation**
  `Kannada-alphabet-tta.gif` (41 frames at 10 fps), read frame by frame from
  a mirror copy of the GIF (Commons is not reachable from the authoring
  sandbox). The copy matches the pixel and byte size noted for the Commons
  file (231×208, 105 KB); no frame count was noted to compare. The frames
  show that "tta" draws ಠ, not ಟ ("ta") or ಥ ("thha").
- **Runs and lifts.** The round bowl is one run (frames 0–12): down the left
  side from its upper left, round the base, up the right side and back left
  across the top. After a lift the top bar runs left to right into the hook
  (frames 15–22), and after a second lift the dot is set (frames 26–27). Six
  movements in three strokes. Omniglot's copyists most often draw ಠ in three
  strokes (80%), the same count, so no runs are joined.
- **The coordinates are the print glyph's.** The path follows the skeleton of
  the bundled Noto Sans Kannada outline between turning points chosen from
  the animation, and every join is exact. Noto fuses the bowl's top into the
  bar, as in ದ, so the closing movement runs along the lower part of the
  bar's ink and the bar retraces it; the dot is a filled disc, drawn as a
  small closed loop inside it.
- Measured with the default tolerances and no override: `fractionOnInk` is
  1.0000 on every stroke, every join gap is 0, and no ink point is left
  untraced (671 sampled). The filmstrip was rendered and checked by eye; no
  caption needs more than two lines or runs past its panel.
- New tests pin the letter's runs, lifts, labels and Commons URL, and the
  filmstrip's steps and summary. `tests/stroke-ownership.test.ts` was
  re-measured after the captions were settled: keys 499 -> 500, Kannada
  43 -> 44, plus the ordered key hash and the non-Tamil data hash. The
  filmstrip-geometry ledger was regenerated.
