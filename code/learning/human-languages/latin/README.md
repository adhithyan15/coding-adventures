# Latin

A track of the [Human Languages](../README.md) curriculum, on the same
[`HL00`](../../../specs/HL00-human-language-curriculum-framework.md) framework:
one word per lesson, slug ids, atom-first assembly, derivations shown, LaTeX
book.

## What's different about the Latin track

- **A taproot track.** Latin is the ancestor of the Spanish, French, Italian, and
  Portuguese tracks and the single largest source of English vocabulary after its
  Germanic core. Almost every lesson pays a **double dividend**: you learn a Latin
  word *and* see why a dozen English words — and their Romance cousins — look the
  way they do. *Valē* ("goodbye") hands you *value*, *valid*, *valiant*,
  *prevail* in one stroke.
- **Endings, not word order.** Latin carries meaning in its endings, so it can
  drop pronouns entirely (*agō* already means "I do"). The machinery is taught
  one piece at a time, where a real phrase needs it — never a front-loaded
  grammar table.
- **Classical pronunciation**, macrons marking long vowels (*v* = w, *c* always
  hard), and roots traced back toward Proto-Indo-European where the trail is
  clear.

## Progress

- **Chapters 1–36 are authored** as 53 prerequisite-ordered lessons, from
  greetings and numbers through names, time, everyday courtesy, and the honest
  limits of reconstructed conversational phrases.
- **Chapters 2–36 are generated from the same schema-v2 lessons used by Language
  Ladder.** Chapter 1 remains the hand-authored opening; deterministic source
  hashes keep every later app/book chapter pair aligned.
- Every lesson has a shared-spine placement, explicit knowledge boundaries, and
  an effective duration below five minutes.

## Book

The 36-chapter book compiles with XeLaTeX (Latin script, Latin Modern font — no
vendored font needed): `latexmk -xelatex book.tex`.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `LA-C01-salve`); order lives in the book and
`session-map.md`.
