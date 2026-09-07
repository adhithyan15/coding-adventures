### Changed — the last five hand-written pronunciation references are generated

- sanskrit, bengali, gujarati, punjabi and arabic render their
  `appendix-pronunciation.tex` from `<track>/pronunciation-reference.md`. With
  these, every one of the 23 books generates its pronunciation reference and no
  hand-written `.tex` chapter or appendix remains in the corpus.
- **The Gujarati Markdown was missing a whole numbered fact** — the virama and
  the conjunct rule, with સ + ્ + ત → સ્ત. A bare flip would have deleted the rule
  that lets a reader decode any cluster in the book.
- Three more conjunct examples lost the mark they were demonstrating: Sanskrit's
  `न + य`, Bengali's `চ + ছ`. Both are corrected, and each track now shows the
  halant or hasanta glyph it previously only named.
- The Arabic page carried `\emph{\`{}}` and `\emph{\'{}}` — accents applied to
  nothing — in the two entries about ʿayn and hamza. The Markdown writes the real
  characters.
- The Sanskrit preamble gains `\newunicodechar{Ṛ}{\d{R}}`: Latin Modern has the
  lowercase ṛ and not the uppercase, and *Ṛgveda* starts a name.
- `gujarati/pronunciation-reference.md` carried a section titled "A note for the
  LaTeX book". It moves to `gujarati/README.md`; a note about `\gu{}` spans should
  not print between the sounds and the family on a reader's page.
