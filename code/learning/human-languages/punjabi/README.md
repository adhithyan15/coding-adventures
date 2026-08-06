# Punjabi

A track of the [Human Languages](../README.md) curriculum, built the same way
as: one word per lesson, taken apart and traced to its root; the pieces taught
before the whole; and a book you can read straight through.

## What's different about the Punjabi track

- **Gurmukhi, taught inline.** Punjabi is **Indo-Aryan** (like Hindi, Marathi,
  Bengali), written in **Gurmukhi** — "from the mouth of the Guru," the script
  the Sikh Gurus shaped for their scriptures. A vendored Noto Sans Gurmukhi
  font renders it; each word lesson has a *"The letters in this word"* section,
  and a reader who already reads Gurmukhi simply skims it. No reading course.
- **Two vocabularies, front and centre.** The recurring thread is Punjab's
  place on the road between Persia and India: "thank you" is both the
  Sanskritic **dhannavād** and the Perso-Arabic **shukrīā**, and the script
  itself marks borrowed sounds with a *pair bindi* (dot beneath, e.g. **ਸ਼**
  *sha*). The Sikh greeting **sat srī akāl** is taught as a small creed, root
  by root.
- **Grounded against English + Sanskrit + Persian/Arabic**, with the wider
  Indo-European family drawn in where it reaches (*nahīṇ* ← PIE *ne*, English
  *no*).

## Progress

- **Chapter 1 — Greetings** ([`lessons/PA-C01-*`](./lessons/)): sat srī akāl,
  namaste, dhannavād, shukrīā, hāṇ/nahīṇ, practice. In the book.
- **Chapter 2 — Introducing Yourself** ([`lessons/PA-C02-*`](./lessons/)): nāṁ,
  merā, hai, **merā nāṁ … hai**, tū̃/tusī̃, kī, **tuhāḍā nāṁ kī hai?**, khushī,
  practice. Every atom traced; SOV order; two-level "you". In the book.
- **Chapter 3 — How Are You** ([`lessons/PA-C03-*`](./lessons/)): kivēṁ,
  **tusī̃ kivēṁ ho?**, maiṁ, ṭhīk, koī gall nahīṁ, practice. The copula set;
  the *hāṁ* "am"/"yes" homophone. In the book.
- **Chapter 4 — Farewells** ([`lessons/PA-C04-*`](./lessons/)): phir, milāṁge,
  **phir milāṁge**, rabb rākhā ("God keep you"; Arabic + Sanskrit), practice.
  In the book.
- **Chapter 5 — First Verbs** ([`lessons/PA-C05-*`](./lessons/)): bolṇā, **maiṁ
  panjābī boldā hāṁ** (*panj* "five" + *āb* "river"), rahiṇā, kamm karnā,
  practice. The gendered present habitual. In the book.
- **Chapter 6 — Numbers 1–5** ([`lessons/PA-C06-*`](./lessons/)): a short
  counting-and-Gurmukhi lesson followed by a prerequisite-ordered explanation
  of why native Punjabi *panj* and Persian *panj* are convergence, not
  borrowing.

Chapters 1–6 are in the book.

---

## For contributors

Everything below this line is about how the track is built and checked. It is
here for people working on the curriculum; nothing in it is needed to learn the
language.

## What each chapter lets you do

[`chapters.json`](./chapters.json) is the HL05 capability ledger: per chapter, one
first-person can-do sentence and the lesson that pays it off.

- **Chapter 6** — *"I can count from one to five in Gurmukhi, tell the addak
  apart from the tippi, and show that ਪੰਜ is inherited from Sanskrit rather than
  borrowed from the Persian panj in Punjab."* Payoff:
  [`PA-C06-panj-convergence`](./lessons/PA-C06-panj-convergence.md), a task — run
  the convergence argument, with *panjāh* against Hindi *pacās* as the evidence.

Chapters 1–5 are **not in the ledger yet**, and that gap is deliberate. They are
still schema v1, so their lessons declare no knowledge atoms and no payoff there
could honestly claim to assess anything. A placeholder would hide debt the HL05
gap report is meant to surface; the entries land as those chapters migrate.

## Book / fonts

Compiles with XeLaTeX using the **vendored** Noto Sans Gurmukhi font
(`../../_fonts/NotoSansGurmukhi-Static.ttf`). `latexmk -xelatex book.tex`.
Generated Gurmukhi runs use that font while section bookmarks use the lessons'
Latin romanization. A forced six-chapter build is warning-free, and the
handwritten section bookmarks retain readable Gurmukhi plus romanization.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `PA-C01-sat-sri-akal`); order lives in the book and
`session-map.md`.
