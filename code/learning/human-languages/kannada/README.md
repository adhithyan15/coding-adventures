# Kannada

The seventh track of the [Human Languages](../README.md) curriculum, the
second of the four Dravidian tracks (after [Tamil](../tamil/README.md), the
anchor), on the same
[`HL00`](../../../specs/HL00-human-language-curriculum-framework.md) framework:
one word per lesson, slug ids, atom-first assembly, derivations shown, LaTeX
book.

## What's different about the Kannada track

- **The script is taught inside the word lessons — no reading course.** Written
  for someone who may not read a single Kannada letter, each word lesson has a
  *"The letters in this word"* section introducing exactly the letters that
  word needs (ನಮಸ್ಕಾರ brings ನ ಮ ಸ ಕ ರ, the long-ā vowel sign, and the ಸ್ಕ
  *ottakṣara* conjunct). A reader who already reads Kannada skims those notes.
- **Kannada as the Sanskrit-borrowing Dravidian.** The recurring thread is the
  contrast with Tamil: Kannada borrowed *freely* from Sanskrit (*namaskāra*,
  *dhanyavāda*) yet keeps native Dravidian words for the everyday grammar
  (*haudu*, *illa* ≈ Tamil *illai*, *sari* ≈ Tamil *sari*). Each lesson carries
  an **"Across the family"** cognate box (English / Sanskrit / Hindi / Tamil /
  the Dravidian sisters), every form supplied so no prior knowledge is assumed
  — built for pattern-matching across the four Dravidian scripts.
- **Grammar introduced inline**: verb-echo "yes" at *haudu*, negation-by-verb
  at *illa* (the shared Dravidian *il-*), one-word-two-scripts at *sari*.

## Progress

- **Chapter 1 — Greetings** ([`lessons/KA-C01-*`](./lessons/)): namaskāra →
  dhanyavāda → haudu → illa → sari → practice (with the *hōgi baruttēne*
  farewell). Kannada script taught inline; Dravidian cognates traced. In the
  book.
- **Chapter 2 — Introducing Yourself** (planned): *nanna hesaru…* ("my name"),
  and *nīnu* / *nīvu* (familiar / respectful "you").

## Book / fonts

The book compiles with XeLaTeX using the **vendored** Noto Sans Kannada font
(`../../_fonts/`), loaded by relative path — so it builds identically locally
and in CI, no system-font dependency. `latexmk -xelatex book.tex`.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `KA-C01-namaskara`); order lives in the book and
`session-map.md`.
