### Changed — Telugu pins for eight native-lift refits and two-line captions

- `data/scripts/telugu.json`: the records for ద, డ, ణ, బ, ఫ, ఐ, ఋ and ళ now
  pin `penLifts` at 0, 1, 0, 0, 2, 0, 2 and 0, down from 4, 4, 4, 3, 4, 4, 5
  and 3. Their `strokeOrder` lists mark where the pen lifts, and the old
  "restart and" steps of ఐ and ఋ are gone. Each `strokeOrderNote` gives the
  source's movement count and the native-writer stroke count, with HP Labs
  India's share: ద 81% (87 of 107), డ 75% (79 of 105), ణ 79% (81 of 103), బ
  97% (116 of 120), ఫ 78% (80 of 102), ఐ 70% (71 of 102), ఋ 48% (48 of 99,
  the most common count) and ళ 87% (91 of 105). Each `variation` says these
  are counts only, from the LipiTk 4.0 recognizer's prototypes of
  hpl-telugu-iso-char. The Telugu ledger now averages 1.56 lifts per glyph;
  it was 2.16.
- The `strokeOrder` lists of 23 other Telugu records follow their shortened
  filmstrip captions, so every caption fits on two lines.
- `tests/script-inventories/telugu.evidence.ts`: new pins for the eight
  records, and the ఐ and ఋ pins and every renamed `strokeOrder` step are
  updated in place.
- The Telugu filmstrip-geometry ledger, 34 filmstrip SVGs and their figure
  hashes are regenerated.
- Nothing else moves: no book text, narration, modality record,
  owner-evidence digest or filmstrip target count changes.
