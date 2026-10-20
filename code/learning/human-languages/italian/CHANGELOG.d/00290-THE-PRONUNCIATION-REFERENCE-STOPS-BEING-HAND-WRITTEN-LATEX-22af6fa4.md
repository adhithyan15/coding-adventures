## The pronunciation reference stops being hand-written LaTeX

`italian/book/chapters/appendix-pronunciation.tex` was hand-authored. It printed
as a `\chapter*` like any other chapter and was the last file in this track that
the generator did not own. It is now rendered from
`italian/pronunciation-reference.md`, the same path the chinese, japanese,
marwadi, persian, russian and urdu references already take.

**This was authoring, not a format flip.** The Markdown and the LaTeX had drifted
apart in both directions, so the Markdown was reconciled against the page before
the flip rather than after it:

- The hard-*c*/hard-*g* examples (*casa*, *gatto*) were in the LaTeX and not in
  the Markdown. Restored.
- The LaTeX underlined the doubled *tt* of *notte*. Markdown has no
  underline, so they are marked the way every generated chapter in this corpus
  already marks a significant sub-word — bold inside italic — which is what
  `\emph{buon\textbf{a} sera}` does in Chapter 2.
- The Markdown carried two examples the LaTeX did not (*sciare*; *nonno* vs.
  *nono*), and spells *così* with the grave accent Italian actually uses. The
  LaTeX had `cos\'{\i}`, an acute. Both survive into the book.

The chapter title and the contents line keep their separate strings through the
generator's new `shortTitle` field, so the table of contents still reads
"Italian pronunciation (reference)" rather than repeating the em-dashed title.

