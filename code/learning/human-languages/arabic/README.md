# Arabic

The fourth track of the [Human Languages](../README.md) curriculum, on the
same [`HL00`](../../../specs/HL00-human-language-curriculum-framework.md)
framework: one word per lesson, slug ids, atom-first assembly, derivations
shown, LaTeX book.

## What's different about the Arabic track

Arabic doesn't *trace* to roots — its roots are on the **surface**. Nearly
every word is built from a **three-consonant root** carrying a core meaning,
poured into fixed patterns (s-l-m → *salām*/*islām*/*muslim*/*salaam*), which
is the whole curriculum's obsession made literal. So the Arabic track teaches
the **root system** itself as the organizing engine.

Two more things:

- **The script is taught inside the word lessons — no reading course.** Written
  for someone who may not read a single Arabic letter, each word lesson has a
  *"The letters in this word"* section introducing exactly the letters that word
  needs, right to left (سلام brings ا ل م س and the long-ā; مرحبا adds ب ر ح). A
  reader who already reads Arabic skims those notes. (Per `HL00`'s inline-letters
  rule for non-Latin scripts.)
- **Grounded against English + Spanish.** Arabic's long shadow over Spanish is
  a recurring thread: the article **al-** smuggled into English *algebra*/
  *alcohol*, and the sun-letter assimilation you can still hear in Spanish
  *azúcar* (← *as-sukkar*) — every form supplied so no prior Spanish is assumed.
  The Al-Andalus loanwords the Spanish track traces *backward* are met here from
  the source.

## Progress

- **Chapter 1 — Greetings** ([`lessons/AR-C01-*`](./lessons/)): salām →
  marḥaban → al- → as-salāmu ʿalaykum → ṣabāḥ al-khayr → masāʾ al-khayr →
  shukran → practice. The Arabic script is taught inline (RTL, connecting
  letters, dots-on-a-skeleton, the emphatic consonants, ʿayn/hamza), and the
  root engine + attached *al-* are shown as the words are built. In the book.
- **Chapter 2 — Introducing Yourself** ([`lessons/AR-C02-*`](./lessons/)): ism,
  -ī ("my"), **ismī** ("my name is," zero copula), anta/anti (gendered "you"),
  mā, **mā ismuka/ismuki?** ("what's your name?"), tasharrafnā, practice. The
  zero copula (shared with Dravidian) and "you" split by **gender**. In the
  book.

## Book / fonts

The book compiles with XeLaTeX using the **vendored** Noto Naskh Arabic font
(`../../_fonts/`), loaded by relative path — so it builds identically locally
and in CI, no system-font dependency. `latexmk -xelatex book.tex`.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `AR-C01-salam`); order lives in the book and
`session-map.md`.
