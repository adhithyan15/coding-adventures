### Changed — Japanese pins for the katakana and kanji stroke orders

- `tests/script-inventories/japanese.evidence.ts` now walks the katakana
  `コ` and `ヒ` and the kanji `日`, `語` and `本`. Each must cite KanjiVG's
  directed paths for its own code point, with the Unicode name and the
  "Ulrich Apel and contributors, CC BY-SA 3.0" credit, say that only order
  and direction come from that file, and have one stroke-order step per path
  and one pen lift fewer than its paths (1, 1, 3, 13, 4). `語` must say that
  its paths are `言`'s, `五`'s and `口`'s in that order, and that its first
  path runs along the print glyph's short bar.
- `tests/filmstrip-target-counts/japanese.json` moves 70 -> 75:
  JA-W05-go-kanji, -nichi-kanji and -hon-kanji and JA-W06-ko-katakana and
  -hi-katakana now print a filmstrip. The generated LaTeX of chapters 5 and
  6 gains them.
- The owner-evidence digests of the five changed rows are regenerated.
- NOT moved: the letter-anchoring ratchet and the Japanese owner count.
  `言`, `五` and `口` (JA-W05-gen-component, -five-component,
  -mouth-component) get no inventory row, because no word lesson spells them
  on their own and a row would raise Japanese's unread inventory past its
  ceiling of 9. So those three lessons still print no filmstrip.
- Two lessons described a stroke against KanjiVG: JA-W06-hi-katakana drew
  ヒ's short stroke "from right toward left" (it runs left to right, rising a
  little), and JA-W05-five-component gave 五's turn to its second stroke (it
  belongs to the middle bar, the third, as 語's filmstrip shows). Both are
  corrected, with 五's trace line and feedback and the matching recall line in
  JA-C14-go. So the book, narration and modality outputs and hashes of
  chapters 5, 6 and 14 are regenerated. No assessment answer changes and no
  lesson is added, moved or retagged.
