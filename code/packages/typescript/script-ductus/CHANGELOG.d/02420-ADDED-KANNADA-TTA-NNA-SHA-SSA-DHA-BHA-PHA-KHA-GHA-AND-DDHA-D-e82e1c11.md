### Added Kannada tta, nna, sha, ssa, dha, bha, pha, kha, gha and ddha ductus

- `ಟ` (U+0C9F), `ಣ` (U+0CA3), `ಶ` (U+0CB6), `ಷ` (U+0CB7), `ಧ` (U+0CA7),
  `ಭ` (U+0CAD), `ಫ` (U+0CAB), `ಖ` (U+0C96), `ಘ` (U+0C98) and `ಢ`
  (U+0CA2) enter `src/strokes/kannada.ts`. Their bare-consonant rows in
  `kannada.json` (role `syllable`) now carry a `strokeOrderSource`.
- **Order and direction come from Gopala Krishna A's Commons animations**,
  read frame by frame from a mirror copy of each GIF (Commons is not
  reachable from the authoring sandbox): `Kannada-alphabet-ta.gif` (34
  frames), `-nna.gif` (45), `-sha.gif` (31), `-shha.gif` (40),
  `-dhha.gif` (42), `-bha.gif` (33), `-pha.gif` (43), `-kha.gif` (37),
  `-gha.gif` (61) and `-dda.gif` (52), all at 10 fps. The copies of ta,
  nna, sha, shha, dhha, pha and dda match the pixel and byte size noted for
  the Commons file; kha and gha match the noted pixel size, and no byte size
  or frame count was noted for them; no size was noted for bha, and its
  record says so. The frames show that "ta" draws ಟ, "dhha" ಧ and "dda" ಢ.
- **Runs and lifts.** ಟ, ಣ and ಖ are one run with no lift. ಶ draws its body
  from the head round the base up to the bar and lifts once for the hooked
  bar. ಧ and ಢ reuse the ದ and ಡ paths and add a tail after a second lift. ಫ
  reuses ಪ's path and adds a tail; ಷ draws ಪ's body, dot and hooked bar and
  then a slanting stroke; three lifts each.
- **ಭ and ಘ lift less than their animations.** The animations stage ಭ in
  three runs and ಘ in five. Omniglot's non-native copyists (Lake,
  Salakhutdinov & Tenenbaum, Science 350:1332, 2015; MIT), whose counts are
  read only as a ceiling on native lifts, most often draw them in two (55%)
  and four (65%). So ಭ runs back left along its bar after the climb and draws
  it into the hook (one lift, before the tail), and ಘ comes back down its
  middle arm to its foot and rounds the right arm (three lifts). Every
  animated run is still a segment, in order, from its start and in its
  direction.
- **The coordinates are the print glyph's.** Each path follows the skeleton
  of the bundled Noto Sans Kannada outline between turning points chosen from
  the animation, and every join is exact. Where Noto's shape differs, the
  record says so: ಟ's small loop shares its left side with the outer curve,
  so that stretch is drawn twice; ಣ's waist ends in a short tick, run out to
  and back from; ಶ's top is fused into the bar, which retraces it; ಖ's
  strokes cross at the waist, which the path passes twice; dots are filled
  discs, drawn as small closed loops inside them; tails and ಷ's slanting
  stroke are straight bars.
- Measured with the default tolerances and no override: `fractionOnInk` is
  1.0000 on every stroke, every join gap is 0, and no ink point is left
  untraced (802 sampled for ಟ, 802 for ಣ, 714 for ಶ, 856 for ಷ, 813 for ಧ,
  849 for ಭ, 856 for ಫ, 813 for ಖ, 980 for ಘ, 909 for ಢ). The ten
  filmstrips were rendered and checked by eye; two captions were shortened
  so that none needs more than two lines or runs past its panel.
- New tests pin each letter's runs, lifts, labels and Commons URL, and each
  filmstrip's steps and summary. `tests/stroke-ownership.test.ts` was
  re-measured after the last caption was settled: keys 459 -> 469, Kannada
  33 -> 43, plus the ordered key hash and the non-Tamil data hash. The
  filmstrip-geometry ledger was regenerated.
