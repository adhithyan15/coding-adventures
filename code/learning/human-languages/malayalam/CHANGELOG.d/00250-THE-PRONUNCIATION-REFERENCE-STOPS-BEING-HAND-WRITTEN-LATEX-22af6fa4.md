## The pronunciation reference stops being hand-written LaTeX

`malayalam/book/chapters/appendix-pronunciation.tex` was hand-authored and printed as a
`\chapter*`. It is now rendered from `malayalam/pronunciation-reference.md`. The
chapter title, the contents line and the running head keep their three separate
strings, so the head over the page still reads "Malayalam script" rather than
"Pronunciation".

**Restored from the LaTeX before the flip:** **eight worked syllables**, കി കീ
കു കൂ കെ കേ കൊ കോ. The Markdown kept only കാ and reduced the rest to bare signs.
Both are on the page now, and the note that the *e*/*o* signs are written before
their consonant but read after — which the Markdown had and the LaTeX did not —
stays.

**Corrected:** the conjunct examples wrote **സ + ക → സ്ക**, with the chandrakkala
materialising in the answer. Worse here than in the sister tracks, because the
chandrakkala appears nowhere else in the retired file — a reader was told about a
mark they never saw. It now reads **സ് + ക → സ്ക**, and the mark itself is shown
in the fact that names it.

**Kept from the Markdown:** the Tamil cognate for every everyday word — *nandi* =
*naṉṟi*, *illa* = *illai*, *śari* = *sari*, *pōyi varām* ≈ *pōy varugiṟēṉ* —
which is the evidence for the "closest sister of Tamil" claim the LaTeX made
without it.

