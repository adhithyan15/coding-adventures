### Changed — Telugu pins for the nya, tha, ma, tta, dha, bha and ddha pen lifts

- `data/scripts/telugu.json`: the records for ఞ, థ, మ, ట, ధ, భ and ఢ now
  pin `penLifts` at 2, 2, 1, 1, 1, 1 and 2, down from 7, 6, 6, 5, 5, 5 and
  5. Their `strokeOrder` lists mark where the pen lifts. Each
  `strokeOrderNote` gives the source's movement count and the native-writer
  stroke count, with HP Labs India's share: ఞ 89% (91 of 102), థ 78% (83
  of 106), మ 78% (80 of 103), ట 94% (96 of 102), ధ 84% (87 of 104), భ 85%
  (88 of 104) and ఢ 79% (80 of 101). Each `variation` says these are counts
  only, from the LipiTk 4.0 recognizer's prototypes of hpl-telugu-iso-char,
  so the breaks keep the guide's order. The Telugu ledger now averages 2.16
  lifts per glyph; it was 2.84.
- The Telugu filmstrip-geometry ledger, the seven filmstrip SVGs (TE-S128,
  TE-S130, TE-S132, TE-S139, TE-S150, TE-S151 and TE-S152) and their figure
  hashes are regenerated.
- Nothing else moves: no book text, narration, modality record,
  owner-evidence digest or filmstrip target count changes.
