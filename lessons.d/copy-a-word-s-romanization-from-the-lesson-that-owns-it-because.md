---
category: Repo policy / workflow reminders
---

# Copy a word's romanization from the lesson that owns it, because a hand-typed combining mark can fail the glyph gate

Malayalam chapter 86 romanized **സുഹൃത്ത്** as `suhr̥tthinṟe`, using `r` plus
**U+0325 COMBINING RING BELOW**. The full suite failed on `glyph-coverage`:
**every character in a generated book has to render in the book font**, and that
combining mark does not.

Nothing about the spelling looked wrong. It is a legitimate ISO 15919
transliteration of the vocalic r, it displays correctly in a terminal, and it
passed `validate` and all twelve `check:*` gates. Only the full suite, which
walks the generated `.tex` against the font's coverage, caught it.

The corpus already held the answer. `ML-C35-suhruthu`, which teaches the word,
romanizes it **suhṛttŭ** with **U+1E5B LATIN SMALL LETTER R WITH DOT BELOW** — a
single precomposed character the font does cover.

**When a lesson mentions a word another lesson owns, copy that lesson's
`romanization` field rather than typing the transliteration again.** Two
spellings of the same sound in one corpus is a defect even when both are
defensible, and the owning lesson's spelling is the one the rest of the track
has already been checked against.

More generally: prefer a **precomposed** character to a base letter plus a
combining mark anywhere in learner-facing prose. They look identical and only one
of them is guaranteed to print.
