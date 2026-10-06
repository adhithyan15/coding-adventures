### Changed — Japanese pins for the ni, ha, ma, ri and n stroke orders

- `tests/script-inventories/japanese.evidence.ts` names に (U+306B), は
  (U+306F), ま (U+307E), り (U+308A) and ん (U+3093). Their rows had said only
  "authoritative" and cited nothing. Each row must now cite KanjiVG's directed
  paths for its own code point, with the Unicode name and the "Ulrich Apel and
  contributors, CC BY-SA 3.0" credit. It must also say that only order and
  direction come from that file. Pen lifts are pinned at 2, 2, 2, 1 and 0.
  The phrases are checked one by one with `toContain`, as for こ to と.
- The same evidence now walks all 46 basic hiragana, あ to ん with を, and
  requires each row to have a `strokeOrderSource` and a note that is not just
  "authoritative". These five were the last rows without one.
- `tests/filmstrip-target-counts/japanese.json` moves 45 -> 50. The five
  writing lessons that already teach these signs (JA-W01-ha, JA-W01-ni,
  JA-W01-n, JA-W03-ri and JA-W03-ma) now print a filmstrip. Chapters 1, 2, 3
  and 4's generated LaTeX gains those figures.
- The five owner-evidence digests are regenerated, because the record bytes
  changed. The owner declarations do not move: Japanese still owns 70
  letters.
- Three lessons' descriptions of their strokes are corrected where they
  contradicted both KanjiVG and the filmstrip now printed beside them.
  JA-W01-ha and JA-W01-ni said the left vertical ends in a flick to the left;
  it flicks up to the right. JA-W01-ha also had the crossbar crossing the left
  vertical, which it does not, and the loop starting at the crossbar's right
  and closing back up; it starts above the crossbar, comes down through it,
  rounds to the left, comes back up across itself and runs out to the lower
  right. JA-W01-n began ん with "a small tick down-left"; the stroke cuts all
  the way down to the lower left, climbs back up the same line and turns over
  a small hump before its rising finish. Chapters 1 and 2's book hashes and
  narration and those three lessons' modality records are regenerated. No
  lesson's knowledge, activity ids, answers or order changes.
- Nothing else moves. The curriculum digest, the lesson count and the
  reinforcement counts stay the same, because no lesson was added or moved.
