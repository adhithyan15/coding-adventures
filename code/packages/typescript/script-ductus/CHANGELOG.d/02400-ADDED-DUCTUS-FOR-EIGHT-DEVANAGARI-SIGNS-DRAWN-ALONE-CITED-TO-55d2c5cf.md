### Added — Ductus for eight Devanagari signs drawn alone, cited to native writers' traces

- **Eight new Devanagari entries** at the end of `src/strokes/devanagari.ts`,
  keyed `devanagari:<sign>` like the letters: ु (U+0941), ू (U+0942),
  े (U+0947), the anusvara ं (U+0902), the nukta ़ (U+093C), the virama
  ् (U+094D), ृ (U+0943) and the candrabindu ँ (U+0901). Seven are one
  continuous stroke; ँ lifts once, between the crescent and the dot. Each is
  fitted to the bundled Noto Sans Devanagari outline of the sign by itself at
  the default tolerances (every stroke at least 99% on ink, nothing
  untraced), with no overrides.
- **The source.** A new `devanagariMarkSource` reads each entry's citation
  from its mark record in `devanagari.json`. The records cite native writers'
  tablet pen traces in HP Labs India's LipiTk Devanagari recognizer (classes
  50 to 62) for the stroke count, start and direction, with shares: ु one
  stroke in 79 of 83, ू 83 of 83, े 162 of 165, ं 156 of 157, ़ 82 of 83,
  ् 80 of 82, ृ 81 of 83, ँ two strokes in 79 of 82. The model is MIT; the
  data under it is research-only, so only counts and shares are cited and no
  trace was copied.
- **What it does not claim.** The writers wrote each sign alone, without a
  consonant or a headline, so nothing here says when a sign is written
  against its consonant or the headline. े is drawn floating, without the
  headline its foot meets in a word.
- **Left out.** ा, ि, ी, ो and ः: Noto prints each with a short piece of
  headline that the traces never draw, so a path that follows the traces
  leaves 3.5% to 37% of the printed ink untraced, over the 2% limit (ी and ो
  are also weak majorities). ै and ौ: the traces split (34% draw both of
  ै's flags the same way; ौ's commonest count is 42%).
- **Evidence.** `tests/strokes/devanagari-marks.test.ts` (data hashes, lifts,
  movement labels, start and turning geometry, source, and the seven signs
  left out) and `tests/ductusview/devanagari-marks.test.ts` (frames, lifts,
  summary, the sign's own outline in the last frame); the shared honesty
  checks cover them through `tests/strokes/devanagari.test.ts`.
  `tests/stroke-ownership.test.ts`: keys 456 → 464, Devanagari 44 → 52, the
  ordered key hash and the non-Tamil data hash move, with a comment; Tamil and
  both shared-identity values do not.
- **Regenerated ledger.** `filmstrip-geometry.d/devanagari.json` gains the
  eight signs. No existing entry changes.
