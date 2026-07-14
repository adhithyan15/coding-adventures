# Tamil

The sixth track of the [Human Languages](../README.md) curriculum, and the
**anchor** of the four Dravidian tracks (Tamil, Kannada, Telugu, Malayalam), on
the same [`HL00`](../../../specs/HL00-human-language-curriculum-framework.md)
framework: one word per lesson, slug ids, atom-first assembly, derivations
shown, LaTeX book.

## What's different about the Tamil track

- **The script is taught inside the word lessons — no reading course.** Written
  for someone who may not read a single Tamil letter, each word lesson has a
  *"The letters in this word"* section introducing exactly the letters that
  word needs (வணக்கம் brings வ ண க ம, the puḷḷi that removes the inherent
  vowel, and the retroflex ண). A reader who already reads Tamil skims those
  notes. (Per `HL00`'s inline-letters rule for non-Latin scripts.)
- **Tamil as the Dravidian root the others trace back to.** Tamil is the
  oldest-attested Dravidian language and the most protective of its native
  word-stock. The recurring thread is where Tamil kept a home-grown word
  (*vaṇakkam*, *naṉṟi*) while its sisters Kannada, Telugu, and Malayalam
  borrowed the Sanskrit one — each lesson carries an **"Across the family"**
  cognate box (English / Sanskrit / Hindi / the Dravidian sisters), every form
  supplied so no prior knowledge is assumed.
- **Grammar introduced inline**, where a word first needs it: the three-way
  *n*/*l*/*r* distinction at *naṉṟi*, verb-echo "yes" at *ām*, negation-by-verb
  at *illai*, one-letter-many-sounds at *sari*.

## Progress

- **Chapter 1 — Greetings** ([`lessons/TA-C01-*`](./lessons/)): vaṇakkam →
  naṉṟi → ām → illai → sari → practice (with the *pōy varugiṟēṉ* farewell).
  Tamil script taught inline; Dravidian cognates traced. In the book.
- **Chapter 2 — Introducing Yourself** (planned): *eṉ peyar…* ("my name"), and
  *nī* / *nīṅgaḷ* (familiar / respectful "you").

## Book / fonts

The book compiles with XeLaTeX using the **vendored** Noto Sans Tamil font
(`../../_fonts/`), loaded by relative path — so it builds identically locally
and in CI, no system-font dependency. `latexmk -xelatex book.tex`.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `TA-C01-vanakkam`); order lives in the book and
`session-map.md`.
