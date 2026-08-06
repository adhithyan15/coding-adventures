# Latin

A track of the [Human Languages](../README.md) curriculum, built the same way
as: one word per lesson, taken apart and traced to its root; the pieces taught
before the whole; and a book you can read straight through.

## What's different about the Latin track

- **A taproot track.** Latin is the ancestor of the Spanish, French, Italian,
  and Portuguese tracks and the single largest source of English vocabulary
  after its Germanic core. Almost every lesson pays a **double dividend**: you
  learn a Latin word *and* see why a dozen English words — and their Romance
  cousins — look the way they do. *Valē* ("goodbye") hands you *value*,
  *valid*, *valiant*, *prevail* in one stroke.
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
- Every lesson has a shared-spine placement, explicit knowledge boundaries, and
  an effective duration below five minutes.
- **All 36 chapters carry a capability entry** in
  [`chapters.json`](./chapters.json) — no chapter is skipped, and no entry is a
  stub.

---

## For contributors

Everything below this line is about how the track is built and checked. It is
here for people working on the curriculum; nothing in it is needed to learn the
language.

## Chapter capability ledger

[`chapters.json`](./chapters.json) is this track's
[HL05](../../../specs/HL05-chapter-capability-and-step-by-step-shape.md) ledger.
It answers, per chapter, the question the lesson corpus alone cannot: *what can I
do when I finish this?*

Each entry carries:

- **`canDo`** — one first-person sentence, in the reader's own words. "I can wish
  someone a good night in Latin, and say why the wish takes the accusative case."
- **`spineNodes`** — the shared spine nodes this chapter realises, derived from
  the `path` segments in [`curriculum.json`](./curriculum.json). Chapter 1 spans
  four (`SPINE-MEET-GREET`, `SPINE-TAKE-LEAVE`, `SPINE-COURTESY-THANK`,
  `SPINE-RESPOND-BASIC`); most later chapters realise exactly one.
- **`payoff`** — the lesson that delivers the chapter's usable result, its kind
  (`dialogue`, `task`, or `production`), a one-line summary of the actual
  exchange, and the knowledge atoms it exercises.

Two properties of this track are worth stating plainly, because the ledger is
where they become visible:

1. **Only Chapter 1 has a terminal practice lesson.** `LA-C01-practice` is the
   track's single `type: practice` step. Every other chapter's payoff is its last
   lesson by `sequence` — which is genuinely where that chapter's recombination
   and wrap-up recall happen, but is not a dedicated consolidation step. A future
   tranche that adds practice lessons should update the payoff pointers here.
2. **`assesses` is copied from the payoff lesson, never invented.** Each list is
   exactly that lesson's own `practises.knowledge`, so the ledger cannot overstate
   what a chapter delivers. On that basis every Latin chapter's payoff exercises
   100% of the atoms its chapter introduces.

## Book

The 36-chapter book compiles warning-free with XeLaTeX (Latin script, Latin
Modern font — no vendored font needed): `latexmk -xelatex book.tex`.

## Files

- [`lessons/`](./lessons/) · [`chapters.json`](./chapters.json)
  · [`curriculum.json`](./curriculum.json)
  · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `LA-C01-salve`); order lives in the book and
`session-map.md`.
