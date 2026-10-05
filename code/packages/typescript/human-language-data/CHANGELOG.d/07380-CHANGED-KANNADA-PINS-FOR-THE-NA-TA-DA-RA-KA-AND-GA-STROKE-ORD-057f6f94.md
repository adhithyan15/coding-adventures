### Changed — Kannada pins for the na, ta, da, ra, ka and ga stroke orders

- `tests/script-inventories/kannada.evidence.ts` pins the six consonant rows
  ನ, ತ, ದ, ರ, ಕ and ಗ: role `syllable`, pen lifts (1, 1, 1, 1, 3, 1), the
  exact stroke-order sentences, the Commons URL, and the citation's frame
  count and duration. The variation must name the mirror copy and its size,
  and for ತ and ದ it must say that the series files them as "tha" and "dha",
  so a later edit cannot cite the ಟ or ಡ animation by its look-alike slug.
- `tests/filmstrip-target-counts/kannada.json`: 13 -> 22. The new targets
  are KA-S01-letter-na, KA-S01-copy-in-a-word, KA-S01-delayed-copy,
  KA-S01-dictation, KA-S05-letter-ta, KA-S06-letter-da, KA-S07-letter-ra,
  KA-S112-letter-ga and KA-S116-letter-ka.
- Regenerated: the nine filmstrip SVGs, the Kannada figure hashes, the
  books and narration for chapters 1, 3, 5, 6, 7 and 68, and the lesson
  modality of the seven lessons whose writing blocks changed (the six letter
  lessons, whose `Writing:` headings lose "— copy what you see", and
  KA-S135).
