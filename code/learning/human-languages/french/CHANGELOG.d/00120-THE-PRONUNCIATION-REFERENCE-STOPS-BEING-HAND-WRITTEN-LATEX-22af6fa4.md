## The pronunciation reference stops being hand-written LaTeX

`french/book/chapters/appendix-pronunciation.tex` was hand-authored and printed
as a `\chapter*`. It is now rendered from `french/pronunciation-reference.md`.

**Two things were only in the LaTeX** and are restored to the Markdown before
the flip: the *les* → *lay* example of a silent final *s*, and the whole
`double-n-blocks-nasal` rule — that a doubled *nn*/*mm* cancels the nasal, *bon*
(*bõ*) against *bonne* (*bun*). The tag was registered in
`core/sound-tags.d/french.json` and described nowhere; now it is described where
a lesson citing it can be looked up.

**One thing the Markdown had was wrong on a page.** It wrote the nasal of *bien*
in IPA, as *byɛ̃*. U+025B is not in Latin Modern, the book's body font, so the
first compile of the generated page logged fifteen missing-character warnings
and printed holes exactly where the vowel being taught should have been. The
page now uses *byẽ*, the notation the hand-written LaTeX printed (`by\~{e}`) and
the one the rest of this reference already uses.

That needed a second step. `core/main-font-charset.json` records that Latin
Modern has a-, i-, n- and o-with-tilde and **not** e-with-tilde, so U+1EBD would
have been the same hole under a different codepoint — the glyph-coverage gate
caught it, which is what that gate is for. The preamble now carries the escape
hatch that file names for exactly this case:

    \newunicodechar{ẽ}{\~{e}}

so the accent is composed rather than looked up, the way the Chinese book's tone
marks and the Indic romanization marks already are.

The Markdown is otherwise the richer source — it splits *j* from soft *g*, adds
*à*/*ù*, and names each entry with the tag its lessons cite — so the page keeps
everything it had and gains the rest.

