# Sanskrit

A track of the [Human Languages](../README.md) curriculum, on the same
[`HL00`](../../../specs/HL00-human-language-curriculum-framework.md) framework:
one word per lesson, slug ids, atom-first assembly, derivations shown, LaTeX
book.

## What's different about the Sanskrit track

- **A taproot track — like Latin, but pointing east *and* west.** Sanskrit is the
  classical ancestor of the Hindi, Marathi, Punjabi, and Bengali tracks (every
  *namaste* and *dhanyavād* is a worn form of a Sanskrit word met here in full),
  **and** a sister of Latin, Greek, and English. Its roots reach west too — *te*
  ↔ *thee*, *su-* ↔ Greek *eu-*, √gam ↔ *come*, *na* ↔ Latin *nōn* ↔ *no*.
  Learning it ties the two halves of the curriculum together.
- **Devanagari, taught inline** (vendored Noto Sans Devanagari — same font as the
  Hindi/Marathi tracks), with attention to what Sanskrit needs even from a
  Devanagari reader: the *visarga* (ḥ), the vocalic ṛ, conjuncts, and *sandhi*.
- **IAST transliteration** alongside the script, roots traced back toward
  Proto-Indo-European where the trail is clear.

## Progress

- **Chapter 1 — Greetings** ([`lessons/SA-C01-*`](./lessons/)): namaste,
  namaskāraḥ, dhanyavādaḥ, svāgatam, ām/na, practice. In the book.
- **Chapter 2 — Introducing Yourself** (planned): *mama nāma…*, and the Sanskrit
  verb that marks singular, **dual**, and plural.

## Book / fonts

Compiles with XeLaTeX using the **vendored** Noto Sans Devanagari font
(`../../_fonts/NotoSansDevanagari-Static.ttf`). `latexmk -xelatex book.tex`.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `SA-C01-namaste`); order lives in the book and
`session-map.md`.
