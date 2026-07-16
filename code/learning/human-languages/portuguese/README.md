# Portuguese

The eleventh track of the [Human Languages](../README.md) curriculum, on the
same [`HL00`](../../../specs/HL00-human-language-curriculum-framework.md)
framework as [Spanish](../spanish/README.md): one word per lesson, slug ids,
gender-before-nouns, atom-first assembly, derivations shown, LaTeX book.

## What's different about the Portuguese track

Portuguese is the Latin daughter that often wore its words down **most** — the
article *ille* eroded all the way to a single letter, **o**. Grounded against
English + Latin, with Spanish/French/Italian supplied for contrast
(self-contained, no prior knowledge of them assumed). Signature points: Latin
*-ct-* → Portuguese *-it-* (*noite*, like French *nuit*); the strong nasal
vowels (*bom* = *bõ*); and the showpiece — *obrigado / obrigada* ("thanks"),
literally "[I am] obliged" (← Latin *obligātus*, English *obligated*), which
uniquely agrees with the **speaker**, not the listener.

## Progress

- **Chapter 1 — Greetings** ([`lessons/PT-C01-*`](./lessons/)): olá, bom/boa,
  o/a (gender), dia, bom dia, tarde/boa tarde, noite/boa noite,
  obrigado/obrigada, practice. In the book.
- **Chapter 2 — Introducing Yourself** (planned): *o meu nome é… / chamo-me…*,
  *tu* vs. *você*.

## Book

Compiles with XeLaTeX (Latin Modern, no vendored font needed). `latexmk
-xelatex book.tex`.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `PT-C01-noite`); order lives in the book and
`session-map.md`.
