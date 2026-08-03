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
- **Chapter 2 — Introducing Yourself** ([`lessons/BN-C02-*`](./lessons/)): nām,
  āmār, **āmār nām …** ("my name is," zero copula), tumi/āpni (+ tui), ki, **tomār
  nām ki?**, ālāp kore bhālo lāglo, practice. The zero copula; no gender. In the
  book.
- **Chapter 3 — How Are You** ([`lessons/BN-C03-*`](./lessons/)): kemon, **tumi
  kemon āchho?**, āmi (← *asmi* → *am*), bhālo, kono bæpār nā, practice. The verb
  *āchhā* returns for state. In the book.
- **Chapter 4 — Farewells** ([`lessons/BN-C04-*`](./lessons/)): ābār, dækhā hôbe,
  **ābār dækhā hôbe**, kāl dækhā hôbe, practice. The impersonal future. In the
  book.
- **Chapter 5 — First Verbs** ([`lessons/BN-C05-*`](./lessons/)): bôlā, **āmi
  bānglā bôli**, thākā (← *sthā* → *stand/stay*), kāj kôrā, practice. The verb
  never changes for gender. In the book.
- **Chapter 6 — Numbers 1–5** ([`lessons/BN-C06-*`](./lessons/)): *ek, dui, tin,
  chār, pā̃ch*, the chandrabindu, and the conservative vowel in *dui*. In the
  book from the same canonical schema-v2 lesson AST and source hash that
  Language Ladder loads.

## Book / fonts

Compiles with XeLaTeX using the **vendored** Noto Sans Bengali font
(`../../_fonts/NotoSansBengali-Static.ttf`). `latexmk -xelatex book.tex`.
Chapter 6 is generated from the canonical lesson; its Bengali-script runs use
that font while its section bookmarks use authored romanization.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `BN-C01-nomoshkar`); order lives in the book and
`session-map.md`.
