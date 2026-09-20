## The pronunciation reference stops being hand-written LaTeX

`marathi/book/chapters/appendix-pronunciation.tex` was hand-authored and printed
as a `\chapter*`. It is now rendered from `marathi/pronunciation-reference.md`.

**The retired page had an empty script group.** It read "Marathi shares it with
the Dravidian languages to the south (Tamil `\mr{}`*ḷ*, Kannada, etc.)" — a
script command with nothing in it, where a Tamil letter was meant to go. The
Markdown had dropped the naming of Tamil and Kannada altogether, so it is
restored here as prose: "the Dravidian languages to the south — Tamil, Kannada
and the rest". Nothing is lost and no empty group survives.

Two transliterations improve, because the Markdown already had them right: the
anusvāra example is *baraṃ*, not the LaTeX's *barṇ*, and the English loanword is
*bĕnk*, not *bṇk*. The Markdown also adds the present-tense gender example
*yeto* / *yete*.

**The breve needed a font mapping.** `core/main-font-charset.json` records that
Latin Modern has no U+0115, so *bĕnk* would have printed as a hole. The preamble
now carries `\newunicodechar{ĕ}{\u{e}}` beside the four dot-under mappings it
already had — the escape hatch that ledger names for exactly this case. The
glyph-coverage gate is what caught it.

