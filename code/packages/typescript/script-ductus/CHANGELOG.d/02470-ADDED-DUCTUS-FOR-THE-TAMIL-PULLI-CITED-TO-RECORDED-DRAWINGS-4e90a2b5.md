### Added — Ductus for the Tamil pulli, cited to recorded drawings of the 18 consonants

- **A new Tamil owner** for the pulli ் (U+0BCD), the dot that removes a
  consonant's inherent vowel: `src/strokes/tamil/U-BCD.ts`, appended to the
  end of the Tamil tail entries so every existing key keeps its relative
  order. It is one short dab (`penLifts: 0`) inside the bundled Noto Sans
  Tamil disc (radius 67 units, centred at -278, 745), fitted at the default
  tolerances with no overrides: all of the path lies on ink and nothing of
  the disc is left untraced.
- **The source.** Abhinaya Rajarajan's *Varai* (Swift Student Challenge
  2026, repository commit `952294fa`) stores hand-recorded reference drawings
  of the 18 consonants with pulli, க் to ன், in the order they were drawn.
  In all 18 the body is one stroke and the dot is a separate second stroke,
  made after the body, centred above it at 0.47 to 0.65 of its width. An
  earlier reading of Info-farmer's *Writing Tamil* animations on Wikimedia
  Commons agrees that the dot follows the body. The repository has no
  licence, so only these facts are cited and nothing is copied. One writer,
  so the `variation` says confidence is medium, and that the dab's direction
  is not a claim (the recorded dot is about 10 by 13 canvas units).
- **The printed cluster.** Every consonant + pulli glyph in the bundled font
  (`kaprehalftamil` and the rest, from GSUB `haln`) is a composite of the
  unchanged consonant and the unchanged pulli disc, so the dab also sits on
  the dot of every printed cluster.
- Tests: `tests/strokes/tamil/U-BCD.test.ts` (honesty checks, data hash, the
  dab stays inside the disc and runs downward, the citation) and
  `tests/ductusview/tamil/U-BCD.test.ts`. Stroke ownership: keys 534 -> 535,
  Tamil 35 -> 36 and the ordered key hash move (commented); the non-Tamil
  data hash and the shared-identity values do not.
- The filmstrip ledger gains one Tamil entry (்); no existing entry changes.
