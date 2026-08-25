### Added — the ramp now includes the script (HL-C18C)

- Add `measureScriptRamp` to `src/ramp.ts`, and two budgets to `core/chapter-policy.json`:
  `maxNewGlyphsPerLesson` (**3**) and `maxNewScriptSystemsPerLesson` (**1**).
- **The atom budget was measuring one of two burdens.** `maxNewAtomsPerLesson` counts
  units of *meaning*. `HI-W01-shirorekha-na-ma` declares **one** atom and puts **twelve**
  new Devanagari glyphs on the page, and passed cleanly for a whole release. It is not an
  outlier: **61 lessons** exceed three new glyphs and **38 of them declare zero atoms**, so
  they read as maximally gentle while teaching up to a dozen new shapes. Decoding is a
  separate skill on a separate curve, and nothing was watching it.
- **3 is the corpus's own p90**, the same rule that justified `maxNewAtomsPerLesson` — not
  the observed max of 12, because a budget placed at the worst case is not a budget. The
  median non-Latin lesson introduces **zero** new glyphs, so this flags genuine spikes
  rather than taxing ordinary lessons.
- **Target script and the cousin layer are counted separately, and only the first is
  charged.** A Kannada Chapter 1 lesson showing the same word in Devanagari, Tamil, Telugu
  and Malayalam looks like a **34-glyph cliff** when the two are conflated; its actual
  Kannada load is **7**. Sister-script material is context for a reader who already knows a
  relative, and English is the only requirement for each book — so it is reported (119
  lessons, up to 26 foreign glyphs in one) and never penalised. What that footprint
  justifies is keeping the layer visually skippable.
- Counting rules, each load-bearing: charged **once**, in reading order, so revision is
  free; **Latin excluded**, or romanization would swamp the signal; **combining marks
  included**, because an abugida is mostly marks; **script digits included**, because ०१२
  is not readable to someone born to ASCII; and **`Script_Extensions`, not `Script`**,
  because ー is formally `Common` and the narrow property undercounts コーヒー by the
  mark that makes it a long vowel.
- `maxNewScriptSystemsPerLesson: 1` states the rule that you cannot introduce more than
  one script at a time. It flags **5** lessons, all Japanese Chapter 1, which opens kanji
  beside hiragana in its first lesson and adds katakana in its fifth.
- Report-only, per the HL05 precedent: the debt predates the measurement.

