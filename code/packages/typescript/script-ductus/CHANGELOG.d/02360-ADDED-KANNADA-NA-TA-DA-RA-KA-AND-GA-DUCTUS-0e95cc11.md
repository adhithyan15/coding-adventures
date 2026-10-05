### Added Kannada na, ta, da, ra, ka and ga ductus

- `ನ` (U+0CA8), `ತ` (U+0CA4), `ದ` (U+0CA6), `ರ` (U+0CB0), `ಕ` (U+0C95) and
  `ಗ` (U+0C97) enter `src/strokes/kannada.ts`, the first Kannada base
  consonants with a ductus. Their bare-consonant rows in `kannada.json`
  (role `syllable`) now carry a `strokeOrderSource`, which
  `kannadaLetterSource` already looked up in `letters`.
- **Order and direction come from Gopala Krishna A's Commons animations**,
  the series the Kannada vowels cite. Commons is not reachable from the
  authoring sandbox, so each GIF was read frame by frame from a mirror copy
  that matches the Commons file's listed size and frame count:
  `Kannada-alphabet-na.gif` (34 frames), `-tha.gif` (43), `-dha.gif` (39),
  `-ra.gif` (36), `-ka.gif` (44) and `-ga.gif` (37). The series files dental
  ತ and ದ under "tha" and "dha"; "ta" and "da" draw ಟ and ಡ, which the frames
  show plainly.
- **Runs and lifts.** ನ climbs from its lower tail around the left bowl,
  slants into the right bowl and climbs its right side; ತ sweeps the broad
  bowl, arcs left over the top into the inner loop and rises out of it; ದ
  closes its bowl through the middle point and back along the top; ರ closes
  its round bowl counterclockwise; ಗ climbs the left leg, arches over and comes
  down the right leg. Each then lifts once for the top bar, drawn left to
  right into its hook. ಕ takes four runs (three lifts): the round bowl, the
  lower bar, the short link rising from it, and the upper bar with its hook.
- **The coordinates are the print glyph's.** Each path follows the skeleton
  of the bundled Noto Sans Kannada outline between turning points chosen from
  the animation, and every join is exact. Noto fuses the tops of the ದ, ರ, ಕ
  and ಗ bowls into the bar, so the closing movement runs along the lower part
  of that ink and the bar retraces it; ನ's right side meets the bar instead
  of arching over; ತ's arc over the top is its inner loop's upper edge, so
  the loop's right side is retraced on the way up; ಕ's link is a straight
  waist in Noto where the animation bulges right. Each record says so.
- Measured with the default tolerances and no override: `fractionOnInk` is
  1.0000 on every stroke, every join gap is 0, and no ink point is left
  untraced (666 sampled for ನ, 688 for ತ, 754 for ದ, 630 for ರ, 658 for ಕ,
  574 for ಗ). The six filmstrips were rendered and checked by eye; no
  caption needs more than two lines.
- New tests pin each letter's runs, lifts, labels and Commons URL, and each
  filmstrip's steps and summary. `tests/stroke-ownership.test.ts` was
  re-measured after the last caption was settled: keys 439 -> 445, Kannada
  13 -> 19, plus the ordered key hash and the non-Tamil data hash. The
  filmstrip-geometry ledger was regenerated.
