# Portuguese

The eleventh track of the [Human Languages](../README.md) curriculum, built the
same way as [Spanish](../spanish/README.md): one word per lesson, slug ids,
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
  obrigado/obrigada, practice. Legacy canonical lessons; in the book.
- **Chapter 18 — Verbs at the Core** ([`lessons/PT-C18-*`](./lessons/)): the
  seven verbs the shared spine calls core, in nine Portuguese verbs — *ser* and
  *estar*, *ter* and *haver*, *ir*, *vir*, *dizer*, *ver*, and *saber* and
  *conhecer*. This is the track's first realisation of the canonical
  `VERB-*` concepts, so `SPINE-SAY-WHAT-I-DO` stops being wholly omitted.

## Book

It uses Latin Modern for Portuguese and the repository-vendored Noto Naskh
Arabic only for preserved Arabic source forms in the etymology.

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

Chapters **2–18** are authored, which is every Portuguese chapter that owns a
`core/book-generation.json` target. Chapter **1** is deliberately absent: its
lessons are still schema v1 with no declared `practises.knowledge`, so there is
no honest payoff to point at, and stubbing one would destroy the signal the HL05
gap report exists to measure. That absence is tracked debt.

Chapters 2–5 end in a terminal `practice-mix`, which is the payoff. Chapters
6–18 have none, so the payoff is the chapter's last lesson by sequence — the one
carrying its recombination and wrap-up recall. Chapter 2 carries *two*
`practice-mix` lessons; the terminal one, `PT-C02-formal-practice`, is the
payoff, and its narrow practice set is a known representativeness risk (see
`CHANGELOG.md`).

## Files

- [`chapters.json`](./chapters.json) — the HL05 chapter capability ledger.
- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `PT-C01-noite`); order lives in the book and
`session-map.md`.
