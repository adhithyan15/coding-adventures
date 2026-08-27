## 0.22.0 — the independent (word-initial) vowels (syllabary, PR 6)

- **The syllabaries now carry their standalone vowels.** Everything so far was
  consonant syllables (a consonant + a vowel *sign*); but a word that *begins*
  with a vowel writes a different letter — the **independent vowel** (అ *a*,
  ఆ *ā*, ఇ *i* … ఔ *au*, ఋ *r̥*). Without them a vowel-initial word can't be read.
  Browse now shows an **"Independent vowels (word-initial)"** strip above the
  grid for the three Dravidian scripts.
- **Still Unicode-grounded, still additive.** The generator composes each from
  `<SCRIPT> LETTER <V>` (the inherent /a/ is `LETTER A`), romanized in ISO-15919
  from the same vetted vowel table as the signs — never re-typed, so the vocalic
  R is r̥ (r + U+0325), not IAST ṛ. They live in a **separate `independentVowels`
  field**, not mixed into `letters`, so the consonant syllabary — and the
  slow-unlock gate and the matrix that key on it being all-syllables — is
  completely untouched (the generated `letters` are byte-for-byte unchanged).
- **Control test.** Asserts the real Telugu independent vowels are the 13 expected
  glyphs + ISO-15919 romans (role `vowel`, no fabricated ductus, r̥ = r + U+0325),
  all three scripts carry them, and — the control — none leak into `letters`, so
  `isSyllabary` still holds and the matrix still builds its full 35 × 13 grid.

