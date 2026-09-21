## The pronunciation reference stops being hand-written LaTeX

`sanskrit/book/chapters/appendix-pronunciation.tex` was hand-authored and printed as a
`\chapter*`. It is now rendered from `sanskrit/pronunciation-reference.md`.

**Restored from the LaTeX before the flip:** the second visarga example
*rāmaḥ*; the anusvāra's alternative transliteration *ṅ*; that the anusvāra is a
dot *above the line*; that it is very frequent *in Sanskrit*; and *and more* on
the list of modern Indo-Aryan descendants. The Markdown's independent-vowel list
is the complete one — the LaTeX's trailed off after इ *i*.

**Corrected:** the conjunct example read `स् + त → स्त, न + य → न्य`. The second
half lost its halant, so the fact meant to demonstrate that a halant strips the
vowel showed the halant appearing from nowhere. It now reads `न् + य → न्य`, and
the halant glyph ् is shown in the fact that names it.

**The retired page had a garbled transliteration.** It wrote the Rigveda as
`\d{r}\d{g}veda` and glossed it `"R\d{g} Veda"` — a dot under the *g*, where IAST
puts the dot only under the *r*, and a capital R with no dot at all in the gloss.
It also opened that gloss with an ASCII `"`, which LaTeX sets as a *closing*
quote. The Markdown has always spelled it *Ṛgveda*, so all three defects retire
with the file.

Latin Modern has the lowercase ṛ and not the uppercase Ṛ, so the preamble gains
`\newunicodechar{Ṛ}{\d{R}}` alongside the twelve mappings it already carried.
The glyph-coverage gate caught it.

