# Telugu

The eighth track of the [Human Languages](../README.md) curriculum, the third
of the four Dravidian tracks (after [Tamil](../tamil/README.md) the anchor and
[Kannada](../kannada/README.md)), on the same
[`HL00`](../../../specs/HL00-human-language-curriculum-framework.md) framework:
one word per lesson, slug ids, atom-first assembly, derivations shown, LaTeX
book.

## What's different about the Telugu track

- **The script is taught inside the word lessons — no reading course.** Each
  word lesson has a *"The letters in this word"* section introducing exactly
  the letters that word needs (నమస్కారం brings the vowel signs, the స్క
  below-stacking conjunct, and the anusvāra ం). A reader who already reads
  Telugu skims those notes.
- **Telugu as the Sanskritised Dravidian — and the family's odd one out on
  "no."** Like Kannada, Telugu borrowed heavily from Sanskrit (*namaskāram*,
  *dhanyavādamulu*) yet keeps native Dravidian for the everyday grammar. But
  where Tamil, Kannada, and Malayalam all say "no" on the root *il-*, Telugu
  goes its own way with *lēdu* / *kādu* — a reminder the family has real
  branches. Each lesson carries an **"Across the family"** cognate box, every
  form supplied so nothing is assumed.
- **Grammar introduced inline**: agglutination (the plural *-mulu*) at
  *dhanyavādamulu*, yes/no as statements of being at *avunu*, the
  existence-vs-identity split (*lēdu*/*kādu*) at *lēdu*.

## Progress

- **Chapter 1 — Greetings** ([`lessons/TE-C01-*`](./lessons/)): namaskāram →
  dhanyavādamulu → avunu → lēdu → sarē → practice (with the *veḷḷi vastānu*
  farewell). Telugu script taught inline; Dravidian cognates traced. In the
  book.
- **Chapter 2 — Introducing Yourself** ([`lessons/TE-C02-*`](./lessons/)):
  peru, naa, **nā pēru** ("my name is," zero copula), nuvvu/mīru, ēmiṭi,
  **mī pēru ēmiṭi?** ("what's your name?"), santōṣam, practice. Every atom
  traced (*pēru* ← *\*pēr*, twin of Tamil *peyar*; *santōṣam* Sanskrit). In the
  book.

## Book / fonts

The book compiles with XeLaTeX using the **vendored** Noto Sans Telugu font
(`../../_fonts/`), loaded by relative path — so it builds identically locally
and in CI, no system-font dependency. `latexmk -xelatex book.tex`.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `TE-C01-namaskaram`); order lives in the book and
`session-map.md`.
