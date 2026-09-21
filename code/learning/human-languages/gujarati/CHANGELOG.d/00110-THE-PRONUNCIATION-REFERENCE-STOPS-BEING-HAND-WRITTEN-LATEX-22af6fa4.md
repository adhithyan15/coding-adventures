## The pronunciation reference stops being hand-written LaTeX

`gujarati/book/chapters/appendix-pronunciation.tex` was hand-authored and printed as a
`\chapter*`. It is now rendered from `gujarati/pronunciation-reference.md`.

**This Markdown was missing a whole numbered fact.** Its third fact was the
standalone vowels; the LaTeX's third fact was the virama and the conjunct rule,
with the example સ + ્ + ત → સ્ત. Rendering the Markdown as it stood would have
deleted the rule that lets a reader decode any cluster in the book. The conjunct
fact is restored as fact 3 and the standalone vowels move to their own section,
which is the shape every sister track's reference already has.

**Also restored:** that the *i*-sign િ is written *before* the consonant but read
*after* it; that Gujarati *cut the top line away, so its letters stand free*, that
this is *the one thing to remember* and *the fastest way to tell Gujarati from
Hindi*; that ળ is a **retroflex l**, tongue curled back, and *absent from Hindi*;
that the three genders are **Sanskrit's**, with the (m.)/(f.)/(n.) labels on
*sāro / sārī / sārũ*; that the Sanskrit core is *shared with Hindi*; that the
Perso-Arabic layer is *heavy*, that the Portuguese words came *from the coastal
ports*, and that all of it is the mark of *centuries as a great trading language
across the Arabian Sea*. Gandhi is named *Mohandas K. ("Mahatma") Gandhi*, which
is both of the names the two files used.

**One section was removed rather than rendered.** "A note for the LaTeX book"
told an author to keep punctuation outside a `\gu{}` span. That is true and worth
keeping, but this file is now the book's own source, so the note would have
printed on a reader's page between the sounds and the family. It moves to
`gujarati/README.md`, where the people it is addressed to will find it.

