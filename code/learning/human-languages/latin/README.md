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

- **Chapters 1–41 are authored** as 81 prerequisite-ordered lessons, from
  greetings and numbers through names, time, everyday courtesy, the honest
  limits of reconstructed conversational phrases, and the twenty-four verbs the
  language leans on hardest.
- **Chapter 37 gives the track its verbs** — *sum*, *habeō*, *eō*, *veniō*,
  *dīcō*, *videō*, *sciō*, *dō*, one per lesson. Each teaches its six
  present-tense forms and the English words it left behind, and each flags the
  resemblances that are *not* real: English *have* is not descended from
  *habēre*, and English *know* is not descended from *sciō*.
- **Chapters 38 and 39 add eight more, four at a time** — *cōgitō*, *legō*,
  *intellegō*, *scrībō* in chapter 38, then *capiō*, *rogō*, *adiuvō*, *amō*
  in chapter 39. Four rather than eight is deliberate: eight new verbs in one
  sitting is a steeper ramp than two sittings of four, and pages cost nothing.
  This is where Latin's English dividend is largest: *legō* is behind
  *legible*, *lecture*, *collect* and *elegant*; *scrībō* behind *scribe*,
  *describe* and *manuscript*; *capiō* behind *capture*, *receive* and
  *concept*; and *intellegō* is literally *inter* + *legō*, "to read between."
  Each lesson also names the lookalikes that are not relatives — English
  *read*, *write*, *think*, *ask* and *help* are all Germanic, *juvenile* is
  not from *iuvō*, and *capital* is not from *capiō*.
- **Chapters 40 and 41 add the last eight of the core forty** — *audiō*,
  *dormiō*, *sedeō*, *stō* in chapter 40, then *ambulō*, *currō*, *aperiō*,
  *claudō* in chapter 41. Each lesson names the conjugation its verb belongs to
  (fourth, second, first, third) without teaching the whole system, and the two
  chapters carry the richest English dividend in the track: *audiō* behind
  *audio*, *audit* and — through *ob-audīre*, "to listen toward" — *obey*;
  *currō* behind *current*, *curriculum* and *corridor*, and behind *car* a
  second time by way of a Gaulish wagon-word; *claudō* behind *close* itself.
- **Chapter 40 names a distinction the track had been using silently** —
  *inherited* against *borrowed*. English *sit* and *stand* descend from the
  same ancestors as *sedeō* and *stō* without passing through Latin, while
  *session* and *station* were taken out of Latin centuries later. Chapter 41
  then shows the opposite figure: *aperiō* ("cover away") and *operīre*
  ("cover over") are one root under two prefixes, and English holds both halves
  — *aperture* from the first, *cover* and *curfew* from the second.
- **Five chapters end on a dedicated payoff lesson** — 1, 19, 21, 33, and 36.
  Chapters 19, 21, and 36 close on a real Latin exchange assembled only from
  words the reader has already been taught; chapter 33 closes on a sorting task,
  because its material is etymological and would not honestly support a
  conversation.
- **Chapters 2–41 are generated from the same schema-v2 lessons used by Language
  Ladder.** Chapter 1 remains the hand-authored opening; deterministic source
  hashes keep every later app/book chapter pair aligned.
- Every lesson has a shared-spine placement, explicit knowledge boundaries, and
  an effective duration below five minutes.
- **All 41 chapters carry a capability entry** in
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

1. **Five chapters have a terminal consolidation lesson; 32 do not.** Chapters 1,
   19, 21, 33, and 36 own a `practice`/`practice-mix` step. Every other chapter's
   payoff is still its last lesson by `sequence` — which is genuinely where that
   chapter's recombination and wrap-up recall happen, but is not a dedicated
   consolidation step. A future tranche that adds practice lessons should update
   the payoff pointers here.
2. **`assesses` is copied from the payoff lesson, never invented.** Each list is
   exactly that lesson's own `practises.knowledge`, so the ledger cannot overstate
   what a chapter delivers.
3. **A high representativeness share does not mean the payoff is usable.** The
   share was already 100% for all 36 chapters when every payoff was just the
   chapter's last teaching lesson, because that lesson cumulatively practises the
   whole chapter. The measure that actually moved with the four consolidation
   lessons is a different one: how many chapters end on something the reader
   *does*. Chapter 37 is the track's first chapter below 100% — its payoff
   `LA-C37-do` exercises 9 of the chapter's 16 atoms (56%), because it recombines
   all eight verbs but not all eight of their separate etymology atoms. That is
   the honest number: the chapter's payoff is producing the verbs, not reciting
   their word histories.

## Book

The 37-chapter book compiles warning-free with XeLaTeX (Latin script, Latin
Modern font — no vendored font needed): `latexmk -xelatex book.tex`.

## Files

- [`lessons/`](./lessons/) · [`chapters.json`](./chapters.json)
  · [`curriculum.json`](./curriculum.json)
  · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `LA-C01-salve`); order lives in the book and
`session-map.md`.
