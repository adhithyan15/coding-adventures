## 0.25.0 — the same syllable in its sister scripts (syllabary, PR 9)

- **Telugu కి, Kannada ಕಿ, Malayalam കി — side by side.** The three Dravidian
  cousins write one sound three ways, and once you can read one the others are a
  short hop. The Browse detail panel now shows, under **"Same sound, sister
  scripts,"** the selected syllable as the *other* syllabaries write it — turning
  "learn Telugu" into "learn the family" by making the connection visible (the
  spiral model's whole premise: the links between languages are the memory hooks).
- **Grounded, nothing invented.** A new pure `crossScriptSiblings` matches the
  syllable's romanization *exactly* across scripts. That is safe because Telugu /
  Kannada / Malayalam are all emitted by the one generator from the same
  ISO-15919 scheme, so "ki" is byte-identical everywhere; every sibling glyph is
  a real letter already in another script's data, pulled out by that match.
- **Restricted to the fully-syllabic trio.** Only scripts where every letter is a
  `syllable` (the `isSyllabary` predicate) contribute siblings, so Tamil /
  Devanagari / Gujarati — abugidas that model a consonant and a vowel-sign
  separately — are never mis-matched, and alphabets get no sibling row at all. A
  Malayalam-only row (the alveolar **ṉa**) correctly shows no siblings.
- **Control test.** Telugu "ki" resolves to the real Kannada ಕಿ + Malayalam കി
  (and never Telugu itself); an alphabet and the Malayalam-only ṉa row yield
  none; and — the control — the helper is read-only: `letters`, `isSyllabary` and
  the matrix are untouched.

