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

- **Chapter 1 — Greetings** ([`lessons/LA-C01-*`](./lessons/)): salvē, avē, valē,
  grātiās agō, ita/nōn, practice. In the book.
- **Chapter 2 — Introducing Yourself** (planned): *quis es?*, *nōmen mihi est…*,
  the first-person endings that let Latin drop pronouns.

## Book

Compiles with XeLaTeX (Latin script, Latin Modern font — no vendored font
needed). `latexmk -xelatex book.tex`.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `LA-C01-salve`); order lives in the book and
`session-map.md`.
