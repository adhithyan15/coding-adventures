### Added — Tamil vowel signs compose in written order, so sign lessons and words with signs print filmstrips

- **Written order, not typed order.** Unicode stores every vowel sign after
  its consonant, but Tamil writes ெ, ே and ை to the LEFT of the consonant,
  and first. `figure-targets.ts` gains `WRITTEN_SIGN_SIDES` (per script,
  which side of its consonant each sign is written on) and
  `writtenPiecesOf`, which turns one grapheme into its pieces in written
  order: `கை` → ை, க; `கா` → க, ா. ொ and ோ are decomposed (NFD) into their
  left half and ா and drawn around the consonant (`சொ` → ெ, ச, ா); a
  two-part sign taught by itself (`ோ`) becomes a strip of its two halves.
  `writingSequenceOf` uses it for Tamil words and lists; `filmstripCandidates`
  asks for a sequence before a single letter, so a one-grapheme headword
  written in two pieces is a sequence.
- **Every side is cited.** Only Tamil has a table, with six rows. ெ and ே
  rest on Radhakrishnan's *Tamil Script Learners Manual*, Modules 6 and 7
  (already their `compositionSource`); ா, ி, ீ and ை now cite HP Labs
  India's *Lipi Indic Character Recognizers 4.0 User Manual* (signs written
  as distinct characters left or right of the consonant, units written left
  to right). A new test holds the table to those mark records.
- **What stays refused.** Any sign without a row: the pulli ், ு and ூ (they
  have no cited ductus, and ு/ூ fuse with their consonant), ௌ (its right
  half ௗ has no row), and every sign in every other script, which has no
  table. The pairs Unicode fuses into ligatures (டி, டீ, லீ) are refused by
  `FUSED_SIGN_PAIRS`.
- **The figure.** A sequence strip that holds a vowel sign calls its groups
  parts: "How it is written — 4 parts, one after another", "Part 2 of 4 —
  …", "Parts 1 and 3: stroke order after …", and its `<desc>` says the parts
  are in written order and each sign is drawn without its consonant. The
  book caption reads "How மேசை is written, part by part, stroke by stroke".
  A strip with no sign prints byte for byte what it printed before.
- **14 Tamil lessons gained a filmstrip** (Tamil target count 34 → 48): the
  sign lessons TA-S09 (ி), S102 (ோ), S114 (ா), S123 (ே), S128 (ை), S132 (ீ)
  and S133 (ெ), and the words சரி, சொ, போ, மேலே, கடை, மேசை and சரியா.
  Regenerated: the Tamil filmstrip ledger owner, 14 SVGs, the Tamil
  figure-hash owner and 11 Tamil book chapters. Narration and modality do not
  change. The six mark records gain `components`, `strokeOrder`,
  `strokeOrderNote`, `penLifts` and `strokeOrderSource`, and their evidence
  pins move.
