---
category: Testing & coverage
---

# A romanization copied from a script data file can carry a combining mark the book's main font cannot render

`data/scripts/kannada.json` gives the independent vowel ಋ the ISO-15919 sound
`r̥` — a plain `r` followed by **U+0325 COMBINING RING BELOW**. A new lesson
copied that string straight into its `romanization` field and into four places
in its body.

`tests/glyph-coverage.test.ts` failed:

```
kannada/book/chapters/ch78-the-vowels-that-start-a-word.tex U+0325 '̥' (main)
```

The book's main font is Latin Modern Roman, which has no glyph for that
combining mark, so the generated chapter would have printed a hole. A second
assertion in the same file failed for the same reason: it plants a known gap and
asserts the report finds exactly that one, so any real new gap breaks it too.

**The track already had a renderable convention and the data file is not it.**
Every other Kannada and Telugu lesson writes this sound `ṛ` — U+1E5B, a
precomposed *r with dot below*, which the font has. `KA-S131-vowel-sign-vocalic-r`
and `TE-S129-letter-vocalic-r` both use it.

What to do differently:

- **A script data file's `sound`/`romanization` field is reference data, not
  learner-facing prose.** Before copying one into a lesson, check what the
  track's existing lessons for the same sound already print.
- **Prefer precomposed characters over base-plus-combining-mark** in
  transliteration. `ṛ` renders; `r` + U+0325 does not.
- `preamble.tex` has a `\newunicodechar` escape hatch for exactly this problem
  (it already maps `ṉ`, `ṟ`, `ṁ`, `ḻ`, `ḱ`). Adding a mapping is the right fix
  only when no precomposed character exists; when one does, use it.
- **None of the twelve `check:*` gates catch this, and neither does `validate`.**
  Only the full `npm test` does, via `glyph-coverage.test.ts`. A tranche that
  introduces a new transliteration character has not been verified until the
  full suite has run on a quiescent tree.
