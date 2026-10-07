### Changed — Malayalam filmstrips for twenty-two Moag-cited glyphs, and the anusvara written after its base

- `data/scripts/malayalam.json`: the rows of ക യ ഖ ങ ച ഛ ഞ ഥ ധ ഭ ഫ ള and ഏ
  now carry a stroke order (one phrase per movement), a note, `penLifts: 0` and
  a `strokeOrderSource` citing Moag's *Malayalam: A University Course and
  Reference Grammar*, Tables II and IV (facts only; CC BY-NC-SA 4.0). The
  anusvara's mark record gains its own cited stroke order, and its written
  order (base first, then the ring) now cites Moag's അം, where the ring is
  movement 9, instead of Unicode alone. Eight new vowel-sign mark records
  (ാ ി ീ ു ൂ ൃ െ േ) cite Table III; none claims a written order.
- `src/figure-targets.ts`: `WRITTEN_SIGN_SIDES` gains a Malayalam table with
  one row, ം "after". Every other Malayalam sign stays refused inside a word:
  Moag never numbers the consonant against a vowel sign. Dumping the bundled
  font's GSUB lookups that mention the anusvara finds only Vedic-sign
  reorderings, so no fused pair is added.
- `tests/filmstrip-target-counts/malayalam.json` 35 -> 58. The 23 lessons:
  ML-S01, S03, S04, S07, S111, S115, S120, S122, S128, S129, S130, S134, S138,
  S139, S141, S148, W113-kha, W113-lla, W113-nga, W114-chha, W114-dha, the
  list ML-W01-aa-ra-anusvaram (ാ, ര, ം) and the list
  ML-W07-number-words-6-10 (ഏ, ഴ).
- Tests: a new figure-targets case, `malayalam-anusvara-after-its-base`, holds
  the row to the mark record and keeps the vowel signs refused in words; the
  written-order case now expects three tables;
  `tests/script-inventories/malayalam.evidence.ts` pins every new row (lifts,
  movement count, scan URL, citation, variation) and the new marks, in place of
  the old "ക and യ unverified" pin.
- Lesson prose: the 21 of those lessons that still said "this book does not
  yet tell you where to start" now point at the strip, in the wording already
  used for the other strip lessons, so the guard against a strip lesson
  disclaiming its stroke order holds. Every edited lesson got shorter (the
  longest is 228 s computed); no declared duration changed.
- Regenerated: the Malayalam filmstrip geometry, 23 filmstrip SVGs, the
  Malayalam figure hashes, the Malayalam book chapters and their hashes,
  narration and its hashes, and the 21 lessons' modality manifests.
