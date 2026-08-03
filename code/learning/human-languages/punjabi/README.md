# Punjabi

A track of the [Human Languages](../README.md) curriculum, on the same
[`HL00`](../../../specs/HL00-human-language-curriculum-framework.md) framework:
one word per lesson, slug ids, atom-first assembly, derivations shown, LaTeX
book.

## What's different about the Punjabi track

- **Gurmukhi, taught inline.** Punjabi is **Indo-Aryan** (like Hindi, Marathi,
  Bengali), written in **Gurmukhi** — "from the mouth of the Guru," the script
  the Sikh Gurus shaped for their scriptures. A vendored Noto Sans Gurmukhi font
  renders it; each word lesson has a *"The letters in this word"* section, and a
  reader who already reads Gurmukhi simply skims it. No reading course.
- **Two vocabularies, front and centre.** The recurring thread is Punjab's place
  on the road between Persia and India: "thank you" is both the Sanskritic
  **dhannavād** and the Perso-Arabic **shukrīā**, and the script itself marks
  borrowed sounds with a *pair bindi* (dot beneath, e.g. **ਸ਼** *sha*). The Sikh
  greeting **sat srī akāl** is taught as a small creed, root by root.
- **Grounded against English + Sanskrit + Persian/Arabic**, with the wider
  Indo-European family drawn in where it reaches (*nahīṇ* ← PIE *ne*, English
  *no*).

## Progress

- **Chapter 1 — Greetings** ([`lessons/PA-C01-*`](./lessons/)): sat srī akāl,
  namaste, dhannavād, shukrīā, hāṇ/nahīṇ, practice. In the book.
- **Chapter 2 — Introducing Yourself** ([`lessons/PA-C02-*`](./lessons/)): nāṁ,
  merā, hai, **merā nāṁ … hai**, tū̃/tusī̃, kī, **tuhāḍā nāṁ kī hai?**, khushī,
  practice. Every atom traced; SOV order; two-level "you". In the book.
- **Chapter 3 — How Are You** ([`lessons/PA-C03-*`](./lessons/)): kivēṁ, **tusī̃
  kivēṁ ho?**, maiṁ, ṭhīk, koī gall nahīṁ, practice. The copula set; the *hāṁ*
  "am"/"yes" homophone. In the book.
- **Chapter 4 — Farewells** ([`lessons/PA-C04-*`](./lessons/)): phir, milāṁge,
  **phir milāṁge**, rabb rākhā ("God keep you"; Arabic + Sanskrit), practice. In
  the book.
- **Chapter 5 — First Verbs** ([`lessons/PA-C05-*`](./lessons/)): bolṇā, **maiṁ
  panjābī boldā hāṁ** (*panj* "five" + *āb* "river"), rahiṇā, kamm karnā, practice.
  The gendered present habitual. In the book.
- **Chapter 6 — Numbers 1–5** ([`lessons/PA-C06-*`](./lessons/)): a short
  counting-and-Gurmukhi lesson followed by a prerequisite-ordered explanation
  of why native Punjabi *panj* and Persian *panj* are convergence, not borrowing.

Chapters 1–6 are in the book. Chapter 6 is generated from the same canonical
schema-v2 lesson AST and source hashes that Language Ladder loads, while the
first five chapters retain their authored long-form narrative during migration.

## Book / fonts

Compiles with XeLaTeX using the **vendored** Noto Sans Gurmukhi font
(`../../_fonts/NotoSansGurmukhi-Static.ttf`). `latexmk -xelatex book.tex`.
Generated Gurmukhi runs use that font while section bookmarks use the lessons'
Latin romanization.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `PA-C01-sat-sri-akal`); order lives in the book and
`session-map.md`.
