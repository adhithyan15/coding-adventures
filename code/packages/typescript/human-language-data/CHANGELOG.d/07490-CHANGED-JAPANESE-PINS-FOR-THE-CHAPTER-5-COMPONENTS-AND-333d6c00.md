### Changed — Japanese pins for the chapter 5 components 言, 五 and 口

- `tests/script-owner-declarations.test.ts` moves Japanese letters 87 -> 90:
  `言` (U+8A00), `五` (U+4E94) and `口` (U+53E3) get inventory rows and owner
  declarations, and the three new owner-evidence digests are generated.
- `tests/script-inventories/japanese.evidence.ts` walks the three rows the
  way it walks `日`, `語` and `本`: the KanjiVG URL for each code point, the
  Unicode name, the "Ulrich Apel and contributors, CC BY-SA 3.0" credit, one
  stroke-order step per path and one pen lift fewer (6, 3, 2). `言` must
  carry the same note as `語` about its top mark.
- `tests/filmstrip-target-counts/japanese.json` moves 79 -> 82:
  JA-W05-gen-component, -five-component and -mouth-component now print a
  filmstrip.
- **The letter-anchoring ratchet does not move.** Japanese's
  `unreadInventory` stays at 9 (ア イ ト ハ ル ン 今 有 難), because each new
  row's glyph is read in a word headword: three existing word lessons now
  spell their headword with the kanji the reader has written since chapter
  5. JA-C14-go is `五` (was ご), JA-C11-kuchi is `口` (was くち) and
  JA-C28-iu is `言う` (was いう), each showing its kana reading beside it.
  `cold`, `buildsToward` and `unwritten` are unchanged too.
- Regenerated: the Japanese figure hashes (three new filmstrips), the book
  and narration outputs and hashes of chapters 5, 11, 14 and 28, the
  modality records of JA-W05-five-component, JA-C11-kuchi, JA-C14-go and
  JA-C28-iu (written by `generatedModalityOutputs`, only the files whose
  bytes changed).
- NOT moved: the curriculum digest, the lesson count, the reinforcement
  counts, the script-closure and continuity measures. No lesson is added or moved and no atom is
  introduced; JA-C11-kuchi and JA-C28-iu now also practise the mouth and
  speech component atoms they write.
