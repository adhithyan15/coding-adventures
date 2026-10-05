### Changed — Kannada pins for the ca, pa, jha, tha, ma, la, va and ja stroke orders

- `tests/script-inventories/kannada.evidence.ts` pins the eight consonant
  rows ಚ, ಪ, ಝ, ಥ, ಮ, ಲ, ವ and ಜ: role `syllable`, pen lifts (3, 2, 4, 3, 2,
  0, 1, 1), the exact stroke-order sentences, the Commons URL, and the
  citation's frame count and duration. The variation must name the mirror
  copy and its size; for ಚ and ಝ it must say that no listed Commons size was
  available to compare, and for ಥ it must say that the series files it as
  "thha" while "tha" is dental ತ, so a later edit cannot cite the ತ or ಠ
  animation by mistake.
- `tests/filmstrip-target-counts/kannada.json`: 28 -> 36. The new targets
  are KA-S122-letter-ca, KA-S124-letter-pa, KA-S128-letter-jha,
  KA-S129-letter-tha, KA-S134-letter-ma, KA-S137-letter-la, KA-S139-letter-va
  and KA-S150-letter-ja.
- Regenerated: the eight filmstrip SVGs, the Kannada figure hashes, the books
  and narration for chapters 13, 15, 20, 23, 67, 69, 71 and 72, and the
  lesson modality of the eight letter lessons; the four whose `Writing:`
  headings still read "— copy what you see" lose that suffix.
- The duration budget held without raising it. The writing blocks of
  KA-S128 (ಝ) and KA-S129 (ಥ) first pushed both lessons to a computed 320s
  and 317s, over the 300s ceiling. Their pen-lift notes and the "thha" note
  were cut down, and three stroke-order sentences were shortened ("climb its
  right side and curl in"; "lift, then draw the tail downward"), which brings
  them to 297s and 298s.
