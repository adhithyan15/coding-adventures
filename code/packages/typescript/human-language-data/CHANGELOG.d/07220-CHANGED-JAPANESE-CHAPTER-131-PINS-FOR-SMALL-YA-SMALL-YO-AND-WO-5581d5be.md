### Changed — Japanese chapter 131 pins: small ゃ, small ょ and を

- `tests/script-owner-declarations.test.ts`: Japanese owns 62 letters, not
  59. The three new rows are small ゃ (U+3083), small ょ (U+3087) and を
  (U+3092), each with a generated owner-evidence digest.
- `tests/script-inventories/japanese.evidence.ts` names all three. Small ゃ
  and ょ must cite the full-size sign's animation, KanjiVG's file for the
  small code point and the Unicode name, and must say the size adaptation is
  not independent evidence, as small ゅ does. を must cite KanjiVG's directed
  paths and say that only order and direction come from them. The phrases
  are checked one by one with `toContain`, not with one greedy regex.
- The Japanese integration evidence moves from 734 lessons in 130 chapters to
  743 in 131. Every new lesson carries one activity.
- `tests/filmstrip-target-counts/japanese.json` moves 23 -> 26: the three
  writing lessons print their filmstrips.
- Nothing else moves. Every new letter lesson is anchored (cold stays 1,
  builds-toward stays 35, unwritten 0, unread inventory 9). Script closure
  stays at zero violations and zero never-taught glyphs. Japanese stays A1.
  The R2, R3 and R4 misses stay at 459, 245 and 519: the chapter opens 21
  older slots, and its warm-ups serve all 21.
