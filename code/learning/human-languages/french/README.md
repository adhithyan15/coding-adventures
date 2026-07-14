# French

The second track of the [Human Languages](../README.md) curriculum, built on
the same framework as [Spanish](../spanish/README.md) (see
[`HL00`](../../../specs/HL00-human-language-curriculum-framework.md)): one
word per lesson, gone deep; the widest honest web of English cousins; the
cultural/idiomatic *why*; grammar and pronunciation introduced in context,
never front-loaded.

## What's different about the French track

French grounds each word against **English and its closest Romance sibling,
Spanish** — both worn-down Latin. Per `HL00`'s Audience rule, no prior Spanish
is assumed: the text supplies every Spanish form in full, as enrichment, so the
*differences* between the two Latin daughters can become the lesson:

- *día* (Spanish) vs. *jour* (French) — same Latin *dies*, but French detoured
  through *diurnum* (→ English *journal*, *journey*).
- *buenos días* (plural, a fossilized blessing) vs. *bonjour* (singular).
- Latin *-ct-* → Spanish *-ch-* (*noche*) vs. French *-it-* (*nuit*).
- Formal "you": Spanish *usted* (← "your grace") vs. French *vous* (the plural
  "you") — same politeness, different mechanism (coming in Chapter 2).

## Progress

- **Chapter 1 — Greetings**: authored ([`lessons/FR-C01-*`](./lessons/)) —
  salut, bien, bon/bonne, le/la/les (gender), jour, bonjour, soir, bonsoir,
  nuit, bonne nuit, practice. In the book.
- **Chapter 2 — Introducing Yourself**: authored ([`lessons/FR-C02-*`](./lessons/))
  — je, me, (s')appeler, **je m'appelle** ("my name is"), **tu / vous**,
  comment, **comment vous appelez-vous?** ("what's your name?"), enchanté(e),
  practice. In the book. (Every atom — *je*, *me*, *appelle* — traced to its
  root.)
- Later chapters mirror the Spanish roadmap's themes.

## Files

- [`lessons/`](./lessons/) — the deep one-word practice lessons.
- [`pronunciation-reference.md`](./pronunciation-reference.md) — French sounds,
  to look up on demand.
- [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
- [`book/`](./book/) — the LaTeX book (`latexmk -xelatex book.tex`).

Lessons are named by **slug** (e.g. `FR-C01-jour`), never numbered; order
lives in the book (LaTeX auto-numbers) and `session-map.md`.
