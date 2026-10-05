### Changed — Japanese chapter 136 and 137 pins for the z and p rows

- `tests/script-owner-declarations.test.ts`: Japanese owns 85 letters, not
  78. The seven new rows are ぞ (U+305E), ず (U+305A), ぜ (U+305C), ぱ
  (U+3071), ぴ (U+3074), ぷ (U+3077) and ぺ (U+307A), the signs chapters 136
  and 137 write. Each row has a generated owner-evidence digest.
- `tests/script-inventories/japanese.evidence.ts` names all seven. The three
  z signs must record their base sign and the dakuten as components, and the
  base sign in full and then the two short strokes at the upper right as
  stroke order, the form ざ uses. The four p signs must record their base
  sign and the handakuten, and the base sign in full and then a small circle
  at the upper right, the form ぽ uses. None may cite a stroke-order source,
  and each precomposed glyph must be covered by the inventory in its own
  right.
- The Japanese integration evidence moves from 788 lessons in 135 chapters to
  809 in 137. Every new lesson carries one activity.
- `tests/curriculum-digests/japanese.json` is re-measured for the 21 new
  lessons (788 -> 809).
- Nothing else moves. No filmstrip count changes: none of the seven signs has
  a ductus. Every new letter lesson is anchored (cold stays 1, builds-toward
  35, unwritten 0, unread inventory 9). Script closure stays at zero
  violations and zero never-taught glyphs. Japanese stays A1. The R1, R2, R3
  and R4 misses stay at 1, 459, 245 and 519: the two chapters open 43 older
  slots, and their warm-ups serve all 43.
