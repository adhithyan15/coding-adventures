# Malayalam

The ninth track of the [Human Languages](../README.md) curriculum, the last of
the four Dravidian tracks (after [Tamil](../tamil/README.md) the anchor,
[Kannada](../kannada/README.md), and [Telugu](../telugu/README.md)), on the same
[`HL00`](../../../specs/HL00-human-language-curriculum-framework.md) framework:
one word per lesson, slug ids, atom-first assembly, derivations shown, LaTeX
book.

## What's different about the Malayalam track

- **The script is taught inside the word lessons — no reading course.** Each
  word lesson has a *"The letters in this word"* section introducing exactly
  the letters that word needs (നമസ്കാരം brings the chandrakkala, the സ്ക
  conjunct, and the anusvāram ം). A reader who already reads Malayalam skims
  those notes.
- **Malayalam as Tamil's closest sister — with a Sanskrit overlay.** The two
  split only ~1000 years ago, so the everyday core is strikingly Tamil-like:
  *nandi* (= Tamil *naṉṟi*), *illa* (= Tamil *illai*), *śari* (= *sari*), and
  the *pōyi varām* farewell (≈ Tamil *pōy varugiṟēṉ*). Yet a vast Sanskrit
  layer (the *Maṇipravāḷam* tradition) gives it a Sanskrit formal greeting
  (*namaskāram*) and the largest alphabet of the family. Each lesson carries an
  **"Across the family"** cognate box, every form supplied so nothing is
  assumed — completing the four-way pattern-matching across the Dravidian
  scripts.
- **Grammar introduced inline**: yes/no as demonstratives (*athe*/*alla*) at
  *athe*, negation-by-verb (the shared *il-*) at *illa*.

## Progress

- **Chapter 1 — Greetings** ([`lessons/ML-C01-*`](./lessons/)): namaskāram →
  nandi → athe → illa → śari → practice (with the *pōyi varām* farewell).
  Malayalam script taught inline; Dravidian cognates traced. In the book.
- **Chapter 2 — Introducing Yourself** ([`lessons/ML-C02-*`](./lessons/)):
  peru, enṟe, **āṇŭ** ("is" — the copula), **enṟe pēru … āṇŭ** ("my name is"),
  nī/niṅṅaḷ, entŭ, **ninṟe pēru entāṇŭ?** ("what's your name?"), santōṣam,
  practice. The Malayalam standout: it *has* a copula *āṇŭ*, unlike its
  zero-copula Dravidian sisters. In the book.
- **Chapter 3 — How Are You** ([`lessons/ML-C03-*`](./lessons/)): eṅṅane,
  **sukhamāṇō?** ("are you well?"), ñān, sukham, sāramilla, practice. The copula
  *āṇŭ* + question *-ō*; Sanskrit *sukha*/*sāraṁ* on native grammar. In the book.
- **Chapter 4 — Farewells** ([`lessons/ML-C04-*`](./lessons/)): pōkuka/varika,
  **pōyi varāṁ** ("I'll go and come back"), nāḷe kāṇāṁ, vīṇḍuṁ kāṇāṁ, practice.
  The Dravidian promise-of-return goodbye. In the book.
- **Chapter 5 — First Verbs** ([`lessons/ML-C05-*`](./lessons/)): saṁsārikkuka,
  **ñān malayāḷaṁ saṁsārikkunnu**, tāmasikkuka, jōli ceyyuka, practice. The
  *-unnu* present — the verb never changes for person. In the book.

## Book / fonts

The book compiles with XeLaTeX using the **vendored** Noto Sans Malayalam font
(`../../_fonts/`), loaded by relative path — so it builds identically locally
and in CI, no system-font dependency. `latexmk -xelatex book.tex`.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `ML-C01-namaskaram`); order lives in the book and
`session-map.md`.
