# Marathi

The twelfth track of the [Human Languages](../README.md) curriculum, on the same
[`HL00`](../../../specs/HL00-human-language-curriculum-framework.md) framework:
one word per lesson, slug ids, atom-first assembly, derivations shown, LaTeX
book.

## What's different about the Marathi track

- **Devanagari, taught inline — but Marathi, not Hindi.** Marathi is written in
  the same script as Hindi/Sanskrit and reuses the vendored Devanagari font, but
  it is its own **Indo-Aryan** language (of Maharashtra). The recurring thread is
  what makes it distinct: it prefers **namaskār** as the greeting, keeps **three**
  genders (Hindi has two), marks **gender on the verb** even in the present
  (*yeto* m. / *yete* f.), and has an **extra letter ळ** (retroflex *ḷ*) shared
  with the Dravidian south. Each word lesson has a *"The letters in this word"*
  section; a reader who knows Devanagari skims it.
- **Grounded against English + Sanskrit**, with the wider Indo-European family
  drawn in where it reaches (*nāhī* ← PIE *\*ne*, English *no*).

## Progress

- **Chapter 1 — Greetings** ([`lessons/MR-C01-*`](./lessons/)): namaskār,
  dhanyavād, ho, nāhī, baraṃ, yeto/yete (gendered farewell), practice. In the
  book.
- **Chapter 2 — Introducing Yourself** (planned): *mājhe nāv…*, *tū* / *tumhī*.

## Book / fonts

Compiles with XeLaTeX using the **vendored** Noto Sans Devanagari font
(`../../_fonts/`) — the same font as the Hindi track. `latexmk -xelatex book.tex`.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `MR-C01-namaskar`); order lives in the book and
`session-map.md`.
