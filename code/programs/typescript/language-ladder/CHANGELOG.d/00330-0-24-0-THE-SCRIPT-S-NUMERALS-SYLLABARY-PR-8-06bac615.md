## 0.24.0 — the script's numerals (syllabary, PR 8)

- **The syllabaries now carry their own digits.** Reading a language means
  reading its numbers, and Telugu / Kannada / Malayalam write them with distinct
  glyphs, not Western 0-9 (Telugu ౦౧౨౩౪౫౬౭౮౯). Browse now shows a **"Numerals
  (0–9)"** strip for the three Dravidian scripts, each digit tile the glyph over
  its value.
- **Grounded, additive, same pattern as the independent vowels.** The generator
  composes each from `<SCRIPT> DIGIT <ZERO..NINE>` and romanizes it as the digit
  value (the Unicode name fixes it unambiguously — these are decimal digits, no
  guessing). They live in a **separate `digits` field**, not mixed into `letters`,
  so the consonant syllabary and the gate/matrix that key on it being
  all-syllables stay untouched (the generated `letters` and `independentVowels`
  are byte-for-byte unchanged).
- **Control test.** The real Telugu digits are the 10 expected glyphs mapped to
  "0"…"9" (role `digit`, no fabricated ductus), all three scripts carry them, and
  — the control — none leak into `letters`, so `isSyllabary` still holds and the
  matrix is unaffected.

