## Unreleased — the Hindi pronunciation reference stops being hand-written LaTeX

`hindi/book/chapters/appendix-pronunciation.tex` was hand-authored and printed as
a `\chapter*`. It is now rendered from `hindi/pronunciation-reference.md`.

Every claim the LaTeX made survives, checked one at a time: the three facts, the
ten independent vowels, aspiration, retroflex against dental, nukta, and the two
vocabularies. The Markdown is the richer source and adds the क् + र → क्र
conjunct, the anusvāra and candrabindu, four more aspirate pairs, and the term
*tatsama*.

**One thing had to be reshaped before it could be rendered.** The Markdown wrote
the ten *mātrā* signs as a bullet list nested inside numbered fact 2. A
reference's numbered list treats an indented line as a continuation of the item,
so the nested bullet was not a list at all — the compiled page read "it swaps the
*a* for another vowel: **-** ा = ā", with a literal hyphen where the bullet had
been. The signs are now one run separated by middots, which is how the retired
LaTeX set them and how the page reads. Only a compiled page shows this; no text
assertion could.

The chapter title, the contents line and the running head keep their three
separate strings through the generator's `shortTitle` and `runningHead` fields,
so the running head still says "Devanagari" rather than "Pronunciation".
