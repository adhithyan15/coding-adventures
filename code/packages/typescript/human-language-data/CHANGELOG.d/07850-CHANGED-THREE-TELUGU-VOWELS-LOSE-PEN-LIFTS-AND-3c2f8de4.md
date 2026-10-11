### Changed — three Telugu vowels lose pen lifts: ఆ, ఇ and ఏ

- **Records.** `telugu.json` independent vowels ఆ, ఇ and ఏ: `penLifts` 0, 0
  and 1 (were 1, 2 and 2), one `strokeOrder` step per movement of the new
  ductus (3, 4 and 4; ఏ's hook step now begins "lift"), a new
  `strokeOrderNote` with the native count and share, the LipiTk class added
  to each citation after the original source, and a variation note that
  gives the counts and says what they override: ఆ's lift, the start of its
  second run and its right lobe's direction; ఇ's order, direction and two
  lifts; ఏ's movement 3 start, direction and the lift before it.
- **Lessons.** TE-S135, TE-S163 and TE-S166 follow the numbered strip and
  state no lift count, so their prose is unchanged; book text, narration
  and modality records do not move.
- **Pins.** `tests/script-inventories/telugu.evidence.ts` now asserts the
  three records' lift counts, steps, the note's native count and share, the
  LipiTk citation suffix and the override phrase. Regenerated: the Telugu
  filmstrip-geometry ledger, the three filmstrips (TE-S135, TE-S163,
  TE-S166) and their figure hashes.
- **Audit.** Bengali and Devanagari were checked the same way and need no
  change: with the headline left out on both sides, no glyph draws more
  body runs than a 75% majority of its LipiTk class.
