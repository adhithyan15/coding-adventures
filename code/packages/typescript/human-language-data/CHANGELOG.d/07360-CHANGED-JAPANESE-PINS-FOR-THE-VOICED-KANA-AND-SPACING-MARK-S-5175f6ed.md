### Changed — Japanese pins for the voiced kana and spacing mark stroke orders

- `tests/script-inventories/japanese.evidence.ts` now walks all 22 voiced
  kana rows, が to ぽ. Each must cite KanjiVG's directed paths for its own code
  point, with the Unicode name and the "Ulrich Apel and contributors, CC BY-SA
  3.0" credit, say that only order and direction come from that file, and
  describe its mark (the left tick first, or a ring that starts at its foot
  and runs clockwise). Pen lifts are pinned, and the stroke-order list must be
  the base sign's plus two steps for a dakuten or one for a handakuten. The
  two older loops that required these rows to cite nothing are replaced. ぜ,
  ぶ, ぷ, ぼ and ぽ must also say where KanjiVG differs from the base sign's
  Sirgazil-cited row and that the path follows KanjiVG. The marks ゛, ゜ and ー
  must cite 0309b, 0309c and 030fc with 1, 0 and 0 pen lifts.
- `tests/script-owner-declarations.test.ts`: Japanese letters 85 -> 87. だ
  (U+3060) and ど (U+3069), written since chapters 11 and 9 but covered only
  through decomposition, get rows of their own, because a cited stroke order
  and a ductus need a row to belong to. Their owner declarations are added.
- `tests/filmstrip-target-counts/japanese.json` moves 50 -> 70: JA-W09-do,
  JA-W11-da, JA-W134-ba, -be, -bu, -de, JA-W135-bi, -ge, -gi, -gu,
  JA-W136-ze, -zo, -zu, JA-W137-pa, -pe, -pi, -pu, JA-W03-dakuten,
  JA-W18-handakuten and JA-W06-long-mark now print a filmstrip. The generated
  LaTeX of chapters 3, 6, 9, 11, 18 and 134 to 137 gains those figures.
- The owner-evidence digests of the 20 changed voiced rows and the three
  marks are regenerated, and the two new rows' evidence is generated.
- Nothing else moves. No lesson text changes, so the book hashes, narration,
  modality records, curriculum digest, lesson count and reinforcement counts
  stay the same.
