### Changed — Japanese chapter 134 and 135 pins for eight voiced kana

- `tests/script-owner-declarations.test.ts`: Japanese owns 78 letters, not
  70. The eight new rows are で (U+3067), ば (U+3070), べ (U+3079), ぶ
  (U+3076), び (U+3073), ぐ (U+3050), げ (U+3052) and ぎ (U+304E), the voiced
  kana chapters 134 and 135 write. Each row has a generated owner-evidence
  digest.
- `tests/script-inventories/japanese.evidence.ts` names all eight. Each row
  must record its base sign and the dakuten as its components, the base sign
  in full and then the two short strokes at the upper right as its stroke
  order, and no stroke-order source, the same form が, ご, ざ and ぼ use. Each
  precomposed glyph must also be covered by the inventory, because script
  closure counts the precomposed glyph, not its decomposition.
- The Japanese integration evidence moves from 764 lessons in 133 chapters to
  788 in 135. Every new lesson carries one activity.
- `tests/curriculum-digests/japanese.json` is re-measured for the 24 new
  lessons (764 -> 788).
- Nothing else moves. No filmstrip count changes: none of the eight signs has
  a ductus. Every new letter lesson is anchored (cold stays 1, builds-toward
  35, unwritten 0, unread inventory 9). Script closure stays at zero
  violations and zero never-taught glyphs. Japanese stays A1. The R1, R2, R3
  and R4 misses stay at 1, 459, 245 and 519: the two chapters open 46 older
  slots, and their warm-ups serve all 46.
