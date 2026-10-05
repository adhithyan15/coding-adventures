### Changed — Japanese pins for the a, i, u, e, o and ka stroke orders

- `tests/script-inventories/japanese.evidence.ts` names あ (U+3042), い
  (U+3044), う (U+3046), え (U+3048), お (U+304A) and か (U+304B). Their rows
  had said only "authoritative" and cited nothing. Each row must now cite
  KanjiVG's directed paths for its own code point, with the Unicode name and
  the "Ulrich Apel and contributors, CC BY-SA 3.0" credit. It must also say
  that only order and direction come from that file. Pen lifts are pinned at
  2, 1, 1, 1, 2 and 2. The phrases are checked one by one with `toContain`,
  as for き, け, ぬ, へ and ら.
- `tests/filmstrip-target-counts/japanese.json` moves 34 -> 40. The six
  writing lessons that already teach these signs (JA-W01-i, JA-W01-e,
  JA-W03-a, JA-W03-u, JA-W03-ka and JA-W10-o) now print a filmstrip.
  Chapters 1, 3 and 10's generated LaTeX gains those figures.
- The six owner-evidence digests are regenerated, because the record bytes
  changed. The owner declarations do not move: Japanese still owns 70
  letters.
- JA-W03-ka's description of its first two strokes is corrected. It said the
  first stroke goes down and curves right at the foot, and that the second is
  a short vertical falling to the right. KanjiVG and the print glyph both
  draw a bar that turns down the right side and hooks back left, then a long
  stroke falling to the lower left. The new filmstrip shows that, so the
  prose now says it too. Its two feedback lines follow. Chapter 3's book
  hash, narration and that lesson's modality record are regenerated. The
  lesson's knowledge, activities and order are unchanged.
- Nothing else moves. The curriculum digest, the lesson count and the
  reinforcement counts stay the same, because no lesson was added or moved.
