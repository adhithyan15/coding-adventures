# Italian

The tenth track of the [Human Languages](../README.md) curriculum, on the same
[`HL00`](../../../specs/HL00-human-language-curriculum-framework.md) framework
as [Spanish](../spanish/README.md) and [French](../french/README.md): one word
per lesson, slug ids, gender-before-nouns, atom-first assembly, derivations
shown, LaTeX book.

## What's different about the Italian track

Italian is the **closest living child of Latin**, so its words wear their roots
most plainly of the Romance languages. Grounded against English + Latin, with
its sisters Spanish and French supplied for contrast (self-contained, no prior
knowledge of them assumed). The signature contrasts: Latin *-ct-* → Italian
*-tt-* (*notte*) where Spanish went *-ch-* (*noche*) and French *-it-* (*nuit*);
and Italian kept the final vowels French dropped. The showpiece etymology:
*ciao* — the world's most famous "hello" — began as *s-ciào*, "I am your slave"
(← Latin *sclavus*, English *slave*/*Slav*).

## Progress

- **Chapter 1 — Greetings** ([`lessons/IT-C01-*`](./lessons/)): ciao, buono,
  il/la/lo (gender), giorno, buongiorno, sera/buonasera, notte/buonanotte,
  grazie, practice. Hand-authored in the book and still readable as legacy
  curriculum content.
- **Chapters 2–17**: 49 prerequisite-ordered lessons move from wellbeing and
  introductions through farewells, the first verbs, numbers, calendar and time,
  family, food, colours, age, two pasts, *essere/andare*, and the body. All are
  strict schema-v2 lessons and generate the corresponding book chapters from
  the same hashed AST that Language Ladder loads.

## Book

The 104-page volume compiles with XeLaTeX (Latin Modern, no vendored font
needed): `latexmk -xelatex book.tex`. Chapters 2–17 are generated from the
canonical lessons by `npm run generate:books` in the
`human-language-data` package; do not edit those chapter files by hand. A
forced clean build reports zero missing glyphs, layout boxes, duplicate
destinations, Hyperref warnings, or LaTeX warnings.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `IT-C01-giorno`); order lives in the book and
`session-map.md`.
