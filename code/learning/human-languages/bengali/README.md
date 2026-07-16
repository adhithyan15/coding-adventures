# Bengali

A track of the [Human Languages](../README.md) curriculum, on the same
[`HL00`](../../../specs/HL00-human-language-curriculum-framework.md) framework:
one word per lesson, slug ids, atom-first assembly, derivations shown, LaTeX
book.

## What's different about the Bengali track

- **Bengali script, taught inline.** Bengali (*Bāṅlā*) is **Indo-Aryan** (like
  Hindi, Marathi, Punjabi), written in its own script — sister to Devanagari,
  sharing the hanging top line but with rounder shapes. A vendored Noto Sans
  Bengali font renders it; each word lesson has a *"The letters in this word"*
  section, and a reader who already reads Bengali simply skims it. No reading
  course.
- **One sound-shift as the spine.** The recurring thread is Bengali's
  **fingerprint**: the inherent vowel is **ô** (not "a"), *s* tilts to **sh**,
  and *v* collapses to **b** — so Sanskrit *namaskāra* is spoken **nômoshkar**
  and *dhanyavāda* becomes **dhônyobad**. Learn the shift and every
  Sanskrit-derived word becomes legible.
- **Grounded against English + Sanskrit**, with the wider Indo-European family
  drawn in where it reaches (*nā* ← PIE *ne*, English *no*).

## Progress

- **Chapter 1 — Greetings** ([`lessons/BN-C01-*`](./lessons/)): nômoshkar,
  dhônyobad, hyã/nā, āchchhā, āshi (the "I'll come again" goodbye), practice. In
  the book.
- **Chapter 2 — Introducing Yourself** (planned): *āmār nām…*, *tumi* / *āpni*.

## Book / fonts

Compiles with XeLaTeX using the **vendored** Noto Sans Bengali font
(`../../_fonts/NotoSansBengali-Static.ttf`). `latexmk -xelatex book.tex`.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `BN-C01-nomoshkar`); order lives in the book and
`session-map.md`.
