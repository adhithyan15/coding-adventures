### Changed — The filmstrip ledger includes every letter of a sequence filmstrip

- `tests/filmstrip-ledger.test.ts` now reads a sequence target's `letters`
  (human-language-data's new letter-list and separate-letter-word strips) and
  adds each letter to the ledger, never the headword itself. A derived
  sequence contributes its letters only when every one of them has a cited
  ductus, the same all-or-nothing rule the book applies, so the ledger never
  carries a letter for a strip that will not print.
- **Regenerated ledger.** Letters that appear only in lists, with no
  one-letter lesson of their own, gained entries: `arabic.json`,
  `cyrillic.json`, `perso-arabic.json` and `urdu-nastaliq.json` grow. No
  existing entry changes, and no stroke data is added or edited.
