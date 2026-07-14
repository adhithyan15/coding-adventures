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
- **Chapter 2 — Introducing Yourself** (planned): *merā nāṇ…*, *tūṇ* / *tusīṇ*.

## Book / fonts

Compiles with XeLaTeX using the **vendored** Noto Sans Gurmukhi font
(`../../_fonts/NotoSansGurmukhi-Static.ttf`). `latexmk -xelatex book.tex`.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `PA-C01-sat-sri-akal`); order lives in the book and
`session-map.md`.
