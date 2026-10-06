### Added Kannada ca, pa, jha, tha, ma, la, va and ja ductus

- `ಚ` (U+0C9A), `ಪ` (U+0CAA), `ಝ` (U+0C9D), `ಥ` (U+0CA5), `ಮ` (U+0CAE),
  `ಲ` (U+0CB2), `ವ` (U+0CB5) and `ಜ` (U+0C9C) enter
  `src/strokes/kannada.ts`. Their bare-consonant rows in `kannada.json` (role
  `syllable`) now carry a `strokeOrderSource`.
- **Order and direction come from Gopala Krishna A's Commons animations**,
  read frame by frame from a mirror copy of each GIF (Commons is not
  reachable from the authoring sandbox): `Kannada-alphabet-cha.gif` (52
  frames), `-pa.gif` (32), `-jha.gif` (53), `-thha.gif` (43), `-ma.gif`
  (42), `-la.gif` (30), `-va.gif` (32) and `-ja.gif` (37), all at 10 fps.
  The copies of pa, ma, la, va and ja match the pixel and byte size listed for
  the Commons file, and thha matches its listed size and frame count; no
  listed size was available for cha and jha, and their records say so. The
  frames show that "thha" draws dental ಥ ("tha" is ತ).
- **Runs and lifts.** ಲ is one run with no lift. ವ draws its body (the curl,
  the base, the middle point and the right side) and lifts once for the
  hooked bar; ಜ draws ಬ's body and lifts once for the upper arc. ಪ lifts for
  the dot and again for the hooked bar; ಮ draws ವ's body, the hooked bar and
  then its right bowl. ಚ draws ಬ's body, then ಕ's lower bar, short link and
  hooked bar, three lifts. ಥ draws ದ's body, then the hooked bar, the tail and
  the dot, three lifts. ಝ draws ರ's bowl, the hooked bar, its middle and
  right arms and the tail, four lifts.
- **The coordinates are the print glyph's.** Each path follows the skeleton
  of the bundled Noto Sans Kannada outline between turning points chosen from
  the animation, and every join is exact. Where Noto's shape differs, the
  record says so: the right side of ಚ, ಮ and ವ runs into a bar instead of
  curving over; ಝ's and ಥ's bowl tops are fused into the bar, which retraces
  them; ಲ's loop shares its left side with the outer curve; the curled tips of
  ಚ's and ಜ's heads are short wedges; ಪ's and ಥ's dots are filled discs, drawn
  as small closed loops inside them.
- Measured with the default tolerances and no override: `fractionOnInk` is
  1.0000 on every stroke, every join gap is 0, and no ink point is left
  untraced (851 sampled for ಚ, 787 for ಪ, 1258 for ಝ, 850 for ಥ, 1045 for ಮ,
  612 for ಲ, 773 for ವ, 691 for ಜ). The eight filmstrips were rendered and
  checked by eye; four captions were shortened so that none needs more than
  two lines.
- New tests pin each letter's runs, lifts, labels and Commons URL, and each
  filmstrip's steps and summary. `tests/stroke-ownership.test.ts` was
  re-measured after the last caption was settled: keys 451 -> 459, Kannada
  25 -> 33, plus the ordered key hash and the non-Tamil data hash. The
  filmstrip-geometry ledger was regenerated.
