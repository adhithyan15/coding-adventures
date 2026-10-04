### Changed — Japanese chapter 133 pins for ki, ke, nu and he

- `tests/script-owner-declarations.test.ts`: Japanese owns 70 letters, not
  65. Four new rows are き (U+304D), け (U+3051), ぬ (U+306C) and へ
  (U+3078), the last basic hiragana without a writing lesson. The fifth is ら
  (U+3089): chapter 8 has written it since JA-W08-ra, but the inventory never
  had a row for it. Each row has a generated owner-evidence digest.
- `tests/script-inventories/japanese.evidence.ts` names all five. Each must
  cite KanjiVG's directed paths for its own code point, with the Unicode name
  and the "Ulrich Apel and contributors, CC BY-SA 3.0" credit, and must say
  that only order and direction come from them. Pen lifts are pinned at 3, 2,
  1, 0 and 1. The phrases are checked one by one with `toContain`, as for を, そ,
  れ and る.
- The Japanese integration evidence moves from 752 lessons in 132 chapters to
  764 in 133. Every new lesson carries one activity.
- `tests/filmstrip-target-counts/japanese.json` moves 29 -> 34: the four
  new writing lessons print their filmstrips, and so does JA-W08-ra now that
  ら has a ductus. Chapter 8's generated LaTeX gains that one figure.
- `tests/curriculum-digests/japanese.json` is re-measured for the twelve new
  lessons (752 -> 764).
- Nothing else moves. Every new letter lesson is anchored (cold stays 1,
  builds-toward 35, unwritten 0, unread inventory 9). Script closure stays at
  zero violations and zero never-taught glyphs. Japanese stays A1. The R2, R3
  and R4 misses stay at 459, 245 and 519: the chapter opens 23 older slots,
  and its warm-ups serve all 23.
