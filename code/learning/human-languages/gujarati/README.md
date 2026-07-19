# Gujarati

A track of the [Human Languages](../README.md) curriculum, on the same
[`HL00`](../../../specs/HL00-human-language-curriculum-framework.md) framework:
one word per lesson, slug ids, atom-first assembly, every atom traced to its
root, script taught inline, a publishable LaTeX book.

## What's different about the Gujarati track

- **The "headless" script, taught inline.** Gujarati (*gujarātī*) is
  **Indo-Aryan** — a granddaughter of Sanskrit, sister to Hindi and Marathi —
  but its script dropped the **top line** (*shirorekhā*) that Devanagari,
  Bengali, and Gurmukhi hang their letters from. A vendored Noto Sans Gujarati
  font renders it; each word lesson introduces the letters it needs, and a reader
  who already reads Gujarati simply skims. No gated reading course.
- **Three genders.** Where Hindi kept two, Gujarati (like Marathi) keeps
  **three** — masculine / feminine / **neuter** — visible right away in the *-o /
  -ī / -ũ* adjective endings (*sāro / sārī / sārũ*, "good").
- **A copula all its own.** Gujarati's "is" is **chhe** — not Hindi's *hai* nor
  Sanskrit's *asti* — one of the quickest tells that a sentence is Gujarati.
- **The trade-language layer.** Gujaratis, a great seafaring merchant people,
  wove **Perso-Arabic** (and later Portuguese) words into everyday speech —
  even "how are you" is answered with a Persian loan (*majā* ← *maza*).
- The language of **Gandhi**, grounded throughout against English + Sanskrit +
  the other Indo-Aryan tracks.

## Progress

- **Chapter 1 — Greetings** ([`lessons/GU-C01-*`](./lessons/)): namaste, ābhār,
  hā/nā, sārũ, āvjo ("come again"), practice.
- **Chapter 2 — Introducing Yourself** ([`lessons/GU-C02-*`](./lessons/)): nām,
  mārũ, chhe, "my name is…", tũ / tame, shũ, "what's your name?", ānand.
- **Chapter 3 — How Are You** ([`lessons/GU-C03-*`](./lessons/)): kem, "tame kem
  chho?", hũ, majā, vāndho nahī.
- **Chapter 4 — Farewells** ([`lessons/GU-C04-*`](./lessons/)): pāchhā, maḷīshũ,
  "pāchhā maḷīshũ", kāle.
- **Chapter 5 — The First Verbs** ([`lessons/GU-C05-*`](./lessons/)): bolvũ,
  "hũ gujarātī bolũ chhũ", rahevũ, kām karvũ.

All five chapters are in the book.

## Book / fonts

Compiles with XeLaTeX using the **vendored** Noto Sans Gujarati font
(`../../_fonts/NotoSansGujarati-Static.ttf`). `latexmk -xelatex book.tex`.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `GU-C01-namaste`); order lives in the book and
`session-map.md`.
