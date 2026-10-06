### Added — Ductus for six Tamil vowel signs, cited to native writers' pen traces

- **Six new Tamil owners**, one file each like a letter: ா (U+0BBE), ி
  (U+0BBF), ீ (U+0BC0), ெ (U+0BC6), ே (U+0BC7) and ை (U+0BC8) in
  `src/strokes/tamil/`, appended to the Tamil tail entries so every existing
  key keeps its relative order. Each is one continuous stroke of three
  movements (`penLifts: 0`), fitted to the bundled Noto Sans Tamil outline at
  the default tolerances (every stroke 100% on ink, nothing untraced), with
  no overrides.
- **The source.** HP Labs India's Lipi Toolkit *Lipi Indic Character
  Recognizers 4.0* (MIT licence) ships a Tamil recognizer whose model stores
  native writers' tablet pen traces, each resampled to 60 points; classes
  36–38 and 41–43 are these six signs (the user manual's Table 3). The
  stored prototypes give the pen lifts, the start and the turning direction:
  87% (ா) to 100% (ி, ெ, ை) are one stroke; ி, ீ, ெ and ை turn clockwise in
  98–100% of them and ே counterclockwise in 99%. Each `variation` gives the
  counts and says the traces are scaled to a square, so they fix the order
  and direction, not the proportions. The pulli ் and the signs ு and ூ are
  not in the recognizer and stay without a ductus.
- **Evidence.** `tests/strokes/tamil/U-BBE…U-BC8.test.ts` (honesty checks,
  data hash, movement labels, start and turning geometry, source) and
  `tests/ductusview/tamil/U-BBE…U-BC8.test.ts` (one unbroken stroke, three
  frames). `tests/stroke-ownership.test.ts`: keys 439 → 445, Tamil 29 → 35
  and the ordered key hash move, with a comment; the non-Tamil data hash and
  both shared-identity values do not.
- **Regenerated ledger.** `filmstrip-geometry.d/tamil.json` gains the six
  signs, which the book now prints in sign lessons and in words. No existing
  entry changes.
