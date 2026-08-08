# Sanskrit

A track of the [Human Languages](../README.md) curriculum, built the same way
as: one word per lesson, taken apart and traced to its root; the pieces taught
before the whole; and a book you can read straight through.

## What's different about the Sanskrit track

- **A taproot track — like Latin, but pointing east *and* west.** Sanskrit is
  the classical ancestor of the Hindi, Marathi, Punjabi, and Bengali tracks
  (every *namaste* and *dhanyavād* is a worn form of a Sanskrit word met here
  in full), **and** a sister of Latin, Greek, and English. Its roots reach west
  too — *te* ↔ *thee*, *su-* ↔ Greek *eu-*, √gam ↔ *come*, *na* ↔ Latin *nōn* ↔
  *no*. Learning it ties the two halves of the curriculum together.
- **Devanagari, taught inline** (vendored Noto Sans Devanagari — same font as
  the Hindi/Marathi tracks), with attention to what Sanskrit needs even from a
  Devanagari reader: the *visarga* (ḥ), the vocalic ṛ, conjuncts, and *sandhi*.
- **IAST transliteration** alongside the script, roots traced back toward
  Proto-Indo-European where the trail is clear.

## Progress

- **Chapter 1 — Greetings** ([`lessons/SA-C01-*`](./lessons/)): namaste,
  namaskāraḥ, dhanyavādaḥ, svāgatam, ām/na, practice. In the book.
- **Chapter 2 — Introducing Yourself** ([`lessons/SA-C02-*`](./lessons/)):
  nāma, mama, asti, **mama nāma … asti** (Sanskrit keeps the copula),
  bhavān/tvam, kim, **tava nāma kim?**, ānandaḥ, practice. Each atom a *source*
  (→ *name/my/is/what*). In the book.
- **Chapter 3 — How Are You** ([`lessons/SA-C03-*`](./lessons/)): katham,
  **bhavān katham asti?**, aham (← *ego* → *I*), kuśalam, na cintā, practice.
  The copula trio asmi/asi/asti. In the book.
- **Chapter 4 — Farewells** ([`lessons/SA-C04-*`](./lessons/)): gacchāmi (←
  *gam* → *come*), punaḥ, **punar-darśanāya**, śvaḥ (kept distinct from
  *hyaḥ*), practice. The dative case. In the book.
- **Chapter 5 — First Verbs** ([`lessons/SA-C05-*`](./lessons/)): vadāmi (the
  **dual** *vadāvaḥ*), **ahaṁ saṁskṛtaṁ vadāmi**, vasāmi (← *vas* → *was*),
  karomi (← √kṛ), practice. In the book.
- **Chapter 6 — Numbers 1–5** ([`lessons/SA-C06-*`](./lessons/)): the gendered
  forms first, then an east-west sound-law map, then *pañca* in *Punjab*,
  *pentagon*, and the qualified history of *punch*—three prerequisite-ordered
  micro-lessons.
- **Chapter 7 — The Core Verbs** ([`lessons/SA-C07-*`](./lessons/)): asti /
  bhavati, gacchati, āgacchati, khādati, paśyati, jānāti. The **dhātu** (root)
  and its **gaṇa** (present-stem class) taught as the system they are, three of
  the ten classes walked in order, the *upasarga* prefix shown by turning "goes"
  into "comes," and every root followed west — \**es-*/\**bheu-* into English's
  patchwork *am/is/are/be/been*, \**gwem-* into *come* and *advent*, \**spek-*
  into *inspect* and *telescope*, \**gno-* into *know* and *diagnosis*.
  **Fully drivable — all six lessons are voice.**
- **Chapter 8 — The Mind and the Palm Leaf**
  ([`lessons/SA-C08-*`](./lessons/)): cintayati, avagacchati/budhyate, paṭhati,
  likhati. Class 10 plants **-अय-**; "understands" is built out of **अव** and a
  verb already owned; **लिख्** means *scratch* before it means *write*. The
  chapter names **तत्सम** against **तद्भव** so that every claim about a
  descendant says which kind it is. **Fully drivable.**
- **Chapter 9 — Taking, Asking, Helping, Loving**
  ([`lessons/SA-C09-*`](./lessons/)): gṛhṇāti, pṛcchati, sāhāyyaṁ karoti,
  snihyati/priyam. \**ghrebh-* → *grab*; \**prek-* → *pray* and *fragen*;
  सहाय "one who goes with" beside Latin *comes*; \**priHos* → *friend*, *free*,
  *Friday* and, eastward, Hindi *piyā*. **Fully drivable.**

Chapters 1–9 are in the book. Core verb coverage: **14 of the canonical 40**,
including all eight verbs the other fifteen verb-bearing tracks share.

---

## For contributors

Everything below this line is about how the track is built and checked. It is
here for people working on the curriculum; nothing in it is needed to learn the
language.

## What each chapter lets you do

[`chapters.json`](./chapters.json) is the HL05 capability ledger: per chapter, one
first-person can-do sentence and the lesson that pays it off.

- **Chapter 6** — *"I can say the Sanskrit numerals एक to पञ्च with their dual and
  gendered forms, and follow पञ्च outward into Punjab, pentagon, and the disputed
  history of punch."* Payoff:
  [`SA-C06-pancha-travels`](./lessons/SA-C06-pancha-travels.md), a task.

  Its representativeness is 7/15 introduced atoms (0.47), just under the 0.5
  policy floor. This chapter is the widest of the Indic six, and its terminal
  lesson follows the *pañca* thread rather than the dual, the gendered paradigm,
  or the Grimm's-law material. The shortfall is recorded rather than padded away.

- **Chapter 7** — *"I can say that someone is, becomes, goes, comes, eats, sees
  and knows in Sanskrit; I can name each verb by its dhātu rather than by its
  present form, say which of the three gaṇa patterns builds its stem, and point
  at the English word that descends from the same root."* Payoff:
  [`SA-C07-janati`](./lessons/SA-C07-janati.md), a production.

  Representativeness is 12/12 (1.00): the payoff lesson gathers the whole
  chapter, because the chapter is one system taught six times rather than six
  unrelated words.

- **Chapter 8** — *"I can say that someone thinks, understands, reads and writes
  in Sanskrit; … and tell a word worn down by speech (तद्भव) from one carried
  over whole (तत्सम)."* Payoff:
  [`SA-C08-likhati`](./lessons/SA-C08-likhati.md), a production.
  Representativeness is 8/8 (1.00).

- **Chapter 9** — *"I can say that someone takes, asks, helps and loves in
  Sanskrit; … and point at the English word — grab, pray, friend — that each of
  these four roots also produced."* Payoff:
  [`SA-C09-snihyati`](./lessons/SA-C09-snihyati.md), a production.
  Representativeness is 8/8 (1.00); it also reaches back three chapters to
  retrieve Grimm's law, the *pañca* travels and the analogical *f* of *four*,
  which nothing had revisited since Chapter 6.

Chapters 1–5 are **not in the ledger yet**, and that gap is deliberate. They are
still schema v1, so their lessons declare no knowledge atoms and no payoff there
could honestly claim to assess anything. A placeholder would hide debt the HL05
gap report is meant to surface; the entries land as those chapters migrate.

## Book / fonts

Compiles with XeLaTeX using the **vendored** Noto Sans Devanagari font
(`../../_fonts/NotoSansDevanagari-Static.ttf`). `latexmk -xelatex book.tex`.
Generated Devanagari runs use that font while section bookmarks use the
lessons' Latin romanization.

The forced nine-chapter build is warning-free — no overfull or underfull boxes,
no missing characters, no hyperref complaints: chapter-qualified recap anchors,
bookmark-safe Devanagari, natural page bottoms, explicit static-font shapes,
and concise running titles keep the downloadable PDF and its outline clean.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `SA-C01-namaste`); order lives in the book and
`session-map.md`.
