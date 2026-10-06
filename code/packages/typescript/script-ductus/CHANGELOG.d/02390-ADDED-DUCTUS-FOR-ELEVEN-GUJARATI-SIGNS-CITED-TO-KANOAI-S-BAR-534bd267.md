### Added — Ductus for eleven Gujarati signs, cited to KanoAI's barakhadi templates

- **Eleven new Gujarati entries** at the end of `src/strokes/gujarati.ts`,
  keyed `gujarati:<sign>` like the letters: ા (U+0ABE), િ (U+0ABF), ી (U+0AC0),
  ુ (U+0AC1), ૂ (U+0AC2), ે (U+0AC7), ૈ (U+0AC8), ો (U+0ACB), ૌ (U+0ACC), the
  anusvara ં (U+0A82) and the visarga ઃ (U+0A83). ા િ ી ુ ૂ ે and ં are one
  continuous stroke; ૈ, ો and ઃ lift once; ૌ lifts twice. Each is fitted to the
  bundled Noto Sans Gujarati outline of the sign by itself at the default
  tolerances (every stroke 100% on ink, nothing untraced), with no overrides.
- **The source.** A new `gujaratiMarkSource` reads each entry's citation from
  its mark record in `gujarati.json`. The records cite KanoAI's hand-made
  barakhadi centre-line templates (Tejas Gajjar, commit `9d3e294`), whose
  consonant-plus-sign SVGs hold one path per pen-down run in handwriting
  order: the start, direction and lifts agree in every consonant row whose
  consonant keeps its bare outline (32 to 34 of 34). The t30apps records for
  આ એ ઐ ઓ ઔ and HP Labs India's LipiTk Devanagari matra prototypes agree on
  the shared shapes. KanoAI's licence is ambiguous (MIT LICENSE, GNU GPL
  README), so only facts are cited and no template path was copied; each
  `variation` says so.
- **Evidence.** `tests/strokes/gujarati-marks.test.ts` (data hashes, lifts,
  movement labels, start and turning geometry, source) and
  `tests/ductusview/gujarati-marks.test.ts` (frames, lifts, summary, the
  sign's own outline in the last frame); the shared honesty checks cover them
  through `tests/strokes/gujarati.test.ts`. `tests/stroke-ownership.test.ts`:
  keys 445 → 456, Gujarati 44 → 55, the ordered key hash and the non-Tamil
  data hash move, with a comment; Tamil and both shared-identity values do not.
- **Regenerated ledger.** `filmstrip-geometry.d/gujarati.json` gains the nine
  signs the book prints (ૌ and ઃ are in no lesson yet). No existing entry
  changes.
