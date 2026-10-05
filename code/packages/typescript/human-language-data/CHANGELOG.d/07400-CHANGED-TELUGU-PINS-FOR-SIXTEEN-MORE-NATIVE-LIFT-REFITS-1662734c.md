### Changed — Telugu pins for sixteen more native-lift refits

- `data/scripts/telugu.json`: the records for త, న, ప, య, ర, ల, వ, శ, ష,
  హ, ఠ, జ, చ, అ, ఎ and ఒ now pin `penLifts` at 0, 0, 1, 2, 0, 0, 0, 0, 2,
  1, 1, 1, 0, 0, 0 and 0, down from 1, 2, 3, 3, 1, 1, 2, 2, 3, 3, 2, 3, 1,
  1, 1 and 2. Their `strokeOrder` lists mark where the pen lifts, and the
  old "restart and" steps of జ, చ, ఎ and ఒ are gone. Each
  `strokeOrderNote` gives the source's movement count and the native-writer
  stroke count, with HP Labs India's share: త 51% (54 of 105, a bare
  majority), న 92%, ప 76%, య 48% (the most common count), ర 81%, ల 99%,
  వ 75%, శ 93%, ష 69% (ష draws three strokes, which 27% use), హ 74%,
  ఠ 71%, జ 97%, చ 93%, అ 97%, ఎ 86% and ఒ 99%. Each `variation` says
  these are counts only, from the LipiTk 4.0 recognizer's prototypes of
  hpl-telugu-iso-char. The Telugu ledger now averages 1.02 lifts per glyph;
  it was 1.56.
- `tests/script-inventories/telugu.evidence.ts`: new pins for the sixteen
  records, and the అ, ఎ, చ and ఒ pins are updated in place.
- The Telugu filmstrip-geometry ledger, 19 filmstrip SVGs and their figure
  hashes are regenerated.
- Nothing else moves: no book text, narration, modality record,
  owner-evidence digest or filmstrip target count changes.
