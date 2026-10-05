### Changed — Japanese pins for the ko, sa, su, chi and to stroke orders

- `tests/script-inventories/japanese.evidence.ts` names こ (U+3053), さ
  (U+3055), す (U+3059), ち (U+3061) and と (U+3068). Their rows had said only
  "authoritative" and cited nothing. Each row must now cite KanjiVG's directed
  paths for its own code point, with the Unicode name (ち's is U+3061
  HIRAGANA LETTER TI) and the "Ulrich Apel and contributors, CC BY-SA 3.0"
  credit. It must also say that only order and direction come from that file.
  Pen lifts are pinned at 1, 2, 1, 1 and 1. The phrases are checked one by
  one with `toContain`, as for あ to か.
- `tests/filmstrip-target-counts/japanese.json` moves 40 -> 45. The five
  writing lessons that already teach these signs (JA-W01-ko, JA-W01-chi,
  JA-W03-to, JA-W03-sa and JA-W03-su) now print a filmstrip. Chapters 2, 3
  and 4's generated LaTeX gains those figures.
- The five owner-evidence digests are regenerated, because the record bytes
  changed. The owner declarations do not move: Japanese still owns 70
  letters.
- Three lessons' descriptions of their strokes are corrected where they
  contradicted both KanjiVG and the filmstrip now printed beside them.
  JA-W03-sa called さ "two strokes" in its gloss and title while its body
  counts three, and described the second stroke as a short curve below the
  bar falling to the left. It starts above the bar, slants down to the right
  through it and hooks back left; the foot curve opens to the right, not the
  left. JA-W01-chi's body of ち now has its sharp turn and finishes low on the
  left instead of "to the right and up". JA-W03-to said と's two strokes never
  meet. In KanjiVG the first path ends on the second, and the print glyph
  joins them there, so the title, the hook, the guided-practice cue and the
  recall activity now say the strokes touch: its answer moves from "no" to
  "yes", with matching accepted answers and feedback. Chapters 2, 3 and 4's
  book hashes, narration and those three lessons' modality records are
  regenerated. No lesson's knowledge, activity ids or order changes.
- Nothing else moves. The curriculum digest, the lesson count and the
  reinforcement counts stay the same, because no lesson was added or moved.
