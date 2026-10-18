## The pronunciation reference stops being hand-written LaTeX

`latin/book/chapters/appendix-pronunciation.tex` was hand-authored and printed
as a `\chapter*`. It is now rendered from `latin/pronunciation-reference.md`.

**Three claims were only in the LaTeX** and are restored to the Markdown: that
the classical scheme has no "soft" consonants at all (with *Cicero* glossed
against "SIS-er-o", and *s* against "z"), that no accent marks are needed once
you hear vowel length, and that Latin was the speech of Rome, then of an empire,
then of the medieval church and the learned world.

**The retired page had a garbled word.** It printed *vīnv̄…* — `v\=\i n\=v}\ldots`,
a macron over a *v* — immediately before glossing the same word correctly as
*vīnum*. The Markdown has always spelled it *vīnum*, so the defect retires with
the file. Small caps for the inscriptional **I** and **V** become bold capitals,
which Markdown can express and which makes the same point.

