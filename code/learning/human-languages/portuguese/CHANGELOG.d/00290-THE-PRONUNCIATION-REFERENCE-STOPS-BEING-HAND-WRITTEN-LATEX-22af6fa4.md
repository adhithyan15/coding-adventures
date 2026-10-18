## The pronunciation reference stops being hand-written LaTeX

`portuguese/book/chapters/appendix-pronunciation.tex` was hand-authored and
printed as a `\chapter*`. It is now rendered from
`portuguese/pronunciation-reference.md`.

**Two claims were only in the LaTeX and would have been deleted by a bare
flip**, so they were carried into the Markdown first:

- "English has no exact match — hum the vowel", the one piece of advice the
  nasal-vowel entry actually gives a reader who cannot produce the sound.
- The *r* of *rato* described as Scottish as well as French. The Markdown named
  only French.

The Markdown is otherwise the richer of the two — it lists *é* and *ê* among the
accents that mark an exceptional stress, which the LaTeX did not — so the page
gains from the move as well as keeping what it had. The contents line keeps its
own shorter string through the generator's new `shortTitle` field.

