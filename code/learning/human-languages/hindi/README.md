# Hindi

The fifth track of the [Human Languages](../README.md) curriculum, on the
same [`HL00`](../../../specs/HL00-human-language-curriculum-framework.md)
framework: one word per lesson, slug ids, atom-first assembly, derivations
shown, LaTeX book.

## What's different about the Hindi track

Two things shape this track.

- **The script is taught inside the word lessons — no reading course.** Written
  for someone who may not read a single Devanagari letter, each word lesson has
  a *"The letters in this word"* section that introduces exactly the letters
  that word needs (नमस्ते brings न म स त and the *e*-mātrā and a conjunct), so
  you learn to read the word *and* what it means in the same few minutes. A
  reader who already knows Devanagari simply skims those notes. (Per `HL00`'s
  inline-letters rule for non-Latin scripts.)
- **Hindi's double inheritance is the recurring thread.** Its core vocabulary
  is **Sanskrit** (*namaste*, *dhanyavād* — roots *nam* "to bow," *dhanya*
  "worthy"), but centuries of Persian court rule layered on a second,
  **Perso-Arabic** vocabulary — so "thanks" is also *shukriyā* (the same
  Semitic root **sh-k-r** as Arabic *shukran*) and "farewell" is *alvidā*
  (carrying the Arabic article *al-*). Grounded against English + Arabic, the
  track keeps both streams in view.

## Progress

- **Chapter 1 — Greetings** ([`lessons/HI-C01-*`](./lessons/)): namaste →
  namaskār → dhanyavād → shukriyā → alvidā → practice. Devanagari taught
  inline (inherent *a*, mātrā vowel signs, halant + conjuncts, independent
  vowels) and the Sanskrit vs. Perso-Arabic heritage introduced through the
  words themselves. In the book.
- **Chapter 2 — Introducing Yourself** ([`lessons/HI-C02-*`](./lessons/)): nām,
  merā, hai, **merā nām … hai** ("my name is"), āp/tum, kyā, **āpkā nām kyā
  hai?** ("what's your name?"), khushī, practice. Every atom traced (nām ←
  *nāman* → *name*; hai ← *asti* → *is*); SOV order; the three-level "you". In
  the book.
- **Chapter 3 — How Are You** ([`lessons/HI-C03-*`](./lessons/)): kaise, **āp
  kaise haiṁ?**, maiṁ, hūṁ, ṭhīk, āpkā svāgat hai, practice. The copula trio
  hūṁ/hai/haiṁ (← *asmi/asti* → *am/is*); respect-as-plural. In the book.
- **Chapter 4 — Farewells** ([`lessons/HI-C04-*`](./lessons/)): phir, milenge,
  **phir milenge**, kal milte haiṁ (*kal* = tomorrow *and* yesterday), chaltā
  hūṁ, practice. In the book.
- **Chapter 5 — First Verbs** ([`lessons/HI-C05-*`](./lessons/)): bolnā, **maiṁ
  hindī boltā hūṁ**, rahnā, karnā (← √kṛ), practice. The present habitual and
  gender agreement. In the book.
- **Chapters 6–33 — Shared-spine expansion**
  ([`lessons/HI-C06-*`](./lessons/)): forty prerequisite-ordered lessons add
  numbers, yes/no and polite repair, days and time, colours, family, body,
  seasons, food, months, age, weather, animals, and progressively finer
  morning/evening/afternoon register. Every lesson stays below five minutes and
  every chapter is generated into the book from the same canonical source used
  by Language Ladder.
- **Writing companions W01–W05** ([`lessons/HI-W*`](./lessons/)): eleven short
  steps introduce the headline, letter bodies, inherent vowel, mātrās,
  preposed short **ि**, spineless letters, virama, conjuncts, and finally whole
  words. They remain inline in the opening chapters rather than becoming a
  prerequisite alphabet drill.

## Book / fonts

The 33-chapter book compiles with XeLaTeX using **vendored** Noto Sans
Devanagari, Noto Naskh Arabic, and Noto Sans Cyrillic fonts (`../../_fonts/`),
loaded by relative path — so it builds identically locally and in CI, with no
system-font dependency. Chapters 6–33 are generated from canonical lesson ASTs
and checked against Language Ladder source hashes. `latexmk -xelatex book.tex`.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `HI-C01-namaste`); order lives in the book and
`session-map.md`.
