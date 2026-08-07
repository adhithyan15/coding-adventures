# Sanskrit

A track of the [Human Languages](../README.md) curriculum, built the same way
as: one word per lesson, taken apart and traced to its root; the pieces taught
before the whole; and a book you can read straight through.

## What's different about the Sanskrit track

- **A taproot track — like Latin, but pointing east *and* west.** Sanskrit is
  the classical ancestor of the Hindi, Marathi, Punjabi, and Bengali tracks
  (every *namaste* and *dhanyavād* is a worn form of a Sanskrit word met here
  in full), **and** a sister of Latin, Greek, and English. Its roots reach west
  too — *te* ↔ *thee*, *su-* ↔ Greek *eu-*, √gam ↔ *come*, *na* ↔ Latin *nōn* ↔
  *no*. Learning it ties the two halves of the curriculum together.
- **Devanagari, taught inline** (vendored Noto Sans Devanagari — same font as
  the Hindi/Marathi tracks), with attention to what Sanskrit needs even from a
  Devanagari reader: the *visarga* (ḥ), the vocalic ṛ, conjuncts, and *sandhi*.
- **IAST transliteration** alongside the script, roots traced back toward
  Proto-Indo-European where the trail is clear.

## Progress

- **Chapter 1 — Greetings** ([`lessons/SA-C01-*`](./lessons/)): namaste,
  namaskāraḥ, dhanyavādaḥ, svāgatam, ām/na, practice. In the book.
- **Chapter 2 — Introducing Yourself** ([`lessons/SA-C02-*`](./lessons/)):
  nāma, mama, asti, **mama nāma … asti** (Sanskrit keeps the copula),
  bhavān/tvam, kim, **tava nāma kim?**, ānandaḥ, practice. Each atom a *source*
  (→ *name/my/is/what*). In the book.
- **Chapter 3 — How Are You** ([`lessons/SA-C03-*`](./lessons/)): katham,
  **bhavān katham asti?**, aham (← *ego* → *I*), kuśalam, na cintā, practice.
  The copula trio asmi/asi/asti. In the book.
- **Chapter 4 — Farewells** ([`lessons/SA-C04-*`](./lessons/)): gacchāmi (←
  *gam* → *come*), punaḥ, **punar-darśanāya**, śvaḥ (kept distinct from
  *hyaḥ*), practice. The dative case. In the book.
- **Chapter 5 — First Verbs** ([`lessons/SA-C05-*`](./lessons/)): vadāmi (the
  **dual** *vadāvaḥ*), **ahaṁ saṁskṛtaṁ vadāmi**, vasāmi (← *vas* → *was*),
  karomi (← √kṛ), practice. In the book.
- **Chapter 6 — Numbers 1–5** ([`lessons/SA-C06-*`](./lessons/)): the gendered
  forms first, then an east-west sound-law map, then *pañca* in *Punjab*,
  *pentagon*, and the qualified history of *punch*—three prerequisite-ordered
  micro-lessons.

Chapters 1–6 are in the book.

---

## For contributors

Everything below this line is about how the track is built and checked. It is
here for people working on the curriculum; nothing in it is needed to learn the
language.

## What each chapter lets you do

[`chapters.json`](./chapters.json) is the HL05 capability ledger: per chapter, one
first-person can-do sentence and the lesson that pays it off.

- **Chapter 6** — *"I can say the Sanskrit numerals एक to पञ्च with their dual and
  gendered forms, and follow पञ्च outward into Punjab, pentagon, and the disputed
  history of punch."* Payoff:
  [`SA-C06-pancha-travels`](./lessons/SA-C06-pancha-travels.md), a task.

  Its representativeness is 7/15 introduced atoms (0.47), just under the 0.5
  policy floor. This chapter is the widest of the Indic six, and its terminal
  lesson follows the *pañca* thread rather than the dual, the gendered paradigm,
  or the Grimm's-law material. The shortfall is recorded rather than padded away.

Chapters 1–5 are **not in the ledger yet**, and that gap is deliberate. They are
still schema v1, so their lessons declare no knowledge atoms and no payoff there
could honestly claim to assess anything. A placeholder would hide debt the HL05
gap report is meant to surface; the entries land as those chapters migrate.

## Book / fonts

Compiles with XeLaTeX using the **vendored** Noto Sans Devanagari font
(`../../_fonts/NotoSansDevanagari-Static.ttf`). `latexmk -xelatex book.tex`.
Generated Devanagari runs use that font while section bookmarks use the
lessons' Latin romanization.

The forced six-chapter build is warning-free: chapter-qualified recap anchors,
bookmark-safe Devanagari, natural page bottoms, explicit static-font shapes,
and concise running titles keep the downloadable PDF and its outline clean.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `SA-C01-namaste`); order lives in the book and
`session-map.md`.
