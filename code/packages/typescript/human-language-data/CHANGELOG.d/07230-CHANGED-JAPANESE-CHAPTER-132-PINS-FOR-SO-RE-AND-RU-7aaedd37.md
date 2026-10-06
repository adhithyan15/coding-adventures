### Changed — Japanese chapter 132 pins: そ, れ and る

- `tests/script-owner-declarations.test.ts`: Japanese owns 65 letters, not
  62. The three new rows are そ (U+305D), れ (U+308C) and る (U+308B), each
  with a generated owner-evidence digest.
- `tests/script-inventories/japanese.evidence.ts` names all three. Each must
  cite KanjiVG's directed paths for its own code point, with the Unicode name
  and the "Ulrich Apel and contributors, CC BY-SA 3.0" credit, and must say
  that only order and direction come from them. Pen lifts are pinned at 0, 1
  and 0. The phrases are checked one by one with `toContain`, as for を.
- The Japanese integration evidence moves from 743 lessons in 131 chapters to
  752 in 132. Every new lesson carries one activity.
- `tests/filmstrip-target-counts/japanese.json` moves 26 -> 29: the three
  writing lessons print their filmstrips.
- `tests/curriculum-digests/japanese.json` is re-measured for the nine new
  lessons (743 -> 752).
- Nothing else moves. Every new letter lesson is anchored (cold stays 1,
  builds-toward 35, unwritten 0, unread inventory 9). Script closure stays at
  zero violations and zero never-taught glyphs. Japanese stays A1. The R2, R3
  and R4 misses stay at 459, 245 and 519: the chapter opens 22 older slots,
  and its warm-ups serve all 22.
