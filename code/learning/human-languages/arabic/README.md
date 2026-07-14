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

- **Chapter 1 is a reading course**, written for someone who cannot read a
  single letter. It teaches *reading* incrementally — a few letters per
  lesson, each cashing out in a real, decodable word (ل + ا → **لا**, "no"),
  building up until the greeting set is readable. Vocabulary depth begins in
  Chapter 2. (Per `HL00`'s reading-course rule for non-Latin scripts.)
- **Grounded against English + Spanish.** Arabic's long shadow over Spanish is
  a recurring thread: the article **al-** smuggled into English *algebra*/
  *alcohol*, and the sun-letter assimilation you can still hear in Spanish
  *azúcar* (← *as-sukkar*). The Al-Andalus loanwords the Spanish track traces
  *backward* are met here from the source.

## Progress

- **Chapter 1 — Learning to Read** ([`lessons/AR-C01-read-*`](./lessons/)):
  la → salam → marhaba → sabah/khayr → shukran → alaykum → practice; ~15
  letters, each learned by reading a real word, reaching the full greeting set.
- **Chapter 2 — Greetings** ([`lessons/AR-C02-*`](./lessons/)): the root
  system, marḥaban, salām, al-, as-salāmu ʿalaykum, ṣabāḥ, khayr, ṣabāḥ
  al-khayr, masāʾ, masāʾ al-khayr, shukran, practice — now that you can read
  them. In the book.
- **Chapter 3 — Introducing Yourself** (planned): ismī, the gendered "you".

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
