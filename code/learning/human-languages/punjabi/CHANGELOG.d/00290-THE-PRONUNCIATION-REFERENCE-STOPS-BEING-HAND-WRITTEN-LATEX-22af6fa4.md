## The pronunciation reference stops being hand-written LaTeX

`punjabi/book/chapters/appendix-pronunciation.tex` was hand-authored and printed as a
`\chapter*`. It is now rendered from `punjabi/pronunciation-reference.md`.

**Restored from the LaTeX before the flip:** the date — Guru Angad shaped
Gurmukhi *in the 16th century*, which the Markdown had dropped; the second gloss
on ਖ਼, *x/kh*, where the Markdown gave only *x*; the seven vowel signs shown on
their own (ਾ ਿ ੀ ੁ ੂ ੇ ੋ) beside the attached syllables the Markdown had; and the
*+ nothing* formulation that makes ਅ = *a* a carrier with an empty vowel rather
than a letter that happens to say *a*.

**Kept from the Markdown:** *ġ* for ਗ਼ rather than the LaTeX's dot-below *g*,
which is what ISO 15919 writes, and the third bullet on the two parallel
vocabulary streams (*dhannavād* / *shukrīā*), which the LaTeX did not have.

