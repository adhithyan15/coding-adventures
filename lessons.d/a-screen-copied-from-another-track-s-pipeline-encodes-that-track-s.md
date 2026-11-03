---
category: Repo policy / workflow reminders
---

# A screen copied from another track's pipeline encodes that track's font and conventions, so check each banned character against the target book before trusting it

**What went wrong.** The Punjabi A2 pipeline (`gen.sh`) was cloned from the
Marwadi one. It inherited a character screen that refuses ṁ, ṃ, ɔ, ε, ə,
combining marks and invisible characters, because the Marwadi book font lacks
those glyphs. The Punjabi romanization convention writes word-final ṁ, and its
builder prints it. The first screen-only run therefore stopped on 442 lines,
every one an ṁ, even though the Punjabi book already prints ṁ 453 times with
`missing_character` 0 in the LaTeX baseline. The inherited screen also missed
three characters the Punjabi font really lacks: ̠ (U+0320), ᵃ (U+1D43) and ḇ
(U+1E07), all in quoted CDIAL forms. Only the `glyph-coverage` test caught
them, after a full generation.

**The fix.** ṁ came out of the refused set. The screen now refuses every
combining mark except U+0303, the combining tilde, which the Punjabi book
renders. It also refuses modifier small letters (U+1D2C-1D6A) and the
macron-below letters. The three quotations were reworded without the exotic
characters.

**Do differently.** When cloning a pipeline for a new track, re-derive each
screen from the new track's book, not from the old one's list. Count each
banned character's uses in the target book's existing `.tex` files and its
baseline. Better still, check the book font's coverage of every character in
the specs before generating.
