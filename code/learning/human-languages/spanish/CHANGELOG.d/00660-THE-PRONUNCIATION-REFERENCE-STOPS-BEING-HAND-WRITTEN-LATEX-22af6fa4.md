## The pronunciation reference stops being hand-written LaTeX

`spanish/book/chapters/appendix-pronunciation.tex` was hand-authored and printed
as a `\chapter*`. It is now rendered from `spanish/pronunciation-reference.md`.

Every claim the LaTeX made survives: the five pure vowels, the nine consonants
that differ from English, both default stress rules, the written accent that
overrides them and distinguishes *qué*/*que* and *tú*/*tu*, and the inverted
question and exclamation marks. Checked claim by claim rather than by counting
blocks, then by compiling the book and reading the appendix pages.

The Markdown adds three diphthongs, soft *g*, and the *í* of *días* as a named
case, and gives every entry the tag id its lessons cite in `sounds:` — the only
place those ids are described, since `core/sound-tags.d/spanish.json` lists them
and nothing else.

