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
- **Chapters 2–21**: 64 prerequisite-ordered lessons move from wellbeing and
  introductions through farewells, the first verbs, numbers, calendar and time,
  family, food, colours, age, two pasts, *essere/andare*, the body, and four
  core-verb chapters — the four verbs of the mind (*pensare*, *capire*,
  *leggere*, *scrivere*); the taking/asking/helping set that ends on *mi piace*,
  the verb built backwards; four regular *-are* verbs (*portare*, *comprare*,
  *aspettare*, *incontrare*); and the chapter that splits English "play" in two
  (*giocare* / *suonare*) before *ottenere* and *rispondere*. The track realizes
  **21 of the 40** core verb concepts.

## Book

Chapters 2–21 are generated from the canonical lessons by `npm run
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

Chapters **2–21** are authored, which is every Italian chapter that owns a
`core/book-generation.json` target. Chapter **1** is deliberately absent: its
lessons are still schema v1 with no declared `practises.knowledge`, so there is
no honest payoff to point at, and stubbing one would destroy the signal the HL05
gap report exists to measure. That absence is tracked debt.

Chapters 2–5 end in a terminal `practice-mix`, which is the payoff. Chapters
6–21 have none, so the payoff is the chapter's last lesson by sequence — the one
carrying its recombination and wrap-up recall.

Chapters 18–21 also do what HL09 §7 asks and most of the corpus does not: their
payoffs assess atoms from **earlier** chapters as well as their own. Chapter
18's payoff (*scrivere*) re-practises Chapter 5's *-are* present, Chapter 15's
*passato prossimo*, Chapter 6's *-ct-* → *-tt-* sound-law and Chapter 17's
*mano*; Chapter 19's (*mi piace*) re-practises Chapter 3's *piacere* and *mi
chiamo*, Chapter 9's *stagioni* and Chapter 11's *vino*; Chapter 20's
(*incontrare*) re-practises Chapter 16's *essere*-past and its participle
agreement and Chapter 5's conjugation drill; Chapter 21's (*rispondere*)
re-practises Chapter 15's *passato remoto*, Chapter 17's *mano* principle and
Chapter 19's *prendere* and *chiedere*.

Measured on the committed corpus, none of the 44 atoms these four chapters
introduce misses a reinforcement window, only three are never revisited (the
three the track's final lesson introduces, which nothing later can reach), and
none of the four makes a forward reference. Across the whole track the
never-revisited count is **14 of 156** taught atoms.

## Files

- [`chapters.json`](./chapters.json) — the HL05 chapter capability ledger.
- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `IT-C01-giorno`); order lives in the book and
`session-map.md`.
