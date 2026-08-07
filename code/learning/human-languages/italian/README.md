# Italian

The tenth track of the [Human Languages](../README.md) curriculum, built the
same way as as [Spanish](../spanish/README.md) and
[French](../french/README.md): one word per lesson, taken apart and traced to
its root; every noun's gender learned with the noun; the pieces taught before
the whole; and a book you can read straight through.

## What's different about the Italian track

Italian is the **closest living child of Latin**, so its words wear their roots
most plainly of the Romance languages. Grounded against English + Latin, with
its sisters Spanish and French supplied for contrast (self-contained, no prior
knowledge of them assumed). The signature contrasts: Latin *-ct-* → Italian
*-tt-* (*notte*) where Spanish went *-ch-* (*noche*) and French *-it-*
(*nuit*); and Italian kept the final vowels French dropped. The showpiece
etymology: *ciao* — the world's most famous "hello" — began as *s-ciào*, "I am
your slave" (← Latin *sclavus*, English *slave*/*Slav*).

## Progress

- **Chapter 1 — Greetings** ([`lessons/IT-C01-*`](./lessons/)): ciao, buono,
  il/la/lo (gender), giorno, buongiorno, sera/buonasera, notte/buonanotte,
  grazie, practice. Hand-authored in the book and still readable as legacy
  curriculum content.
- **Chapters 2–17**: 49 prerequisite-ordered lessons move from wellbeing and
  introductions through farewells, the first verbs, numbers, calendar and time,
  family, food, colours, age, two pasts, *essere/andare*, and the body.

## Book

Chapters 2–17 are generated from the canonical lessons by `npm run
generate:books` in the `human-language-data` package; do not edit those chapter
files by hand.

---

## For contributors

Everything below this line is about how the track is built and checked. It is
here for people working on the curriculum; nothing in it is needed to learn the
language.

## Chapter capabilities

[`chapters.json`](./chapters.json) is the track's HL05 capability ledger. Each
entry states, in the reader's own voice, what finishing that chapter lets them
*do* (`canDo`), and names the lesson that proves it (`payoff`) together with the
knowledge atoms that payoff exercises. It is authored intent, not a derived
cache — no validator may rewrite it.

Chapters **2–17** are authored, which is every Italian chapter that owns a
`core/book-generation.json` target. Chapter **1** is deliberately absent: its
lessons are still schema v1 with no declared `practises.knowledge`, so there is
no honest payoff to point at, and stubbing one would destroy the signal the HL05
gap report exists to measure. That absence is tracked debt.

Chapters 2–5 end in a terminal `practice-mix`, which is the payoff. Chapters
6–17 have none, so the payoff is the chapter's last lesson by sequence — the one
carrying its recombination and wrap-up recall.

## Files

- [`chapters.json`](./chapters.json) — the HL05 chapter capability ledger.
- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `IT-C01-giorno`); order lives in the book and
`session-map.md`.
