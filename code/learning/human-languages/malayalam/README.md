# Malayalam

The ninth track of the [Human Languages](../README.md) curriculum, the last of
the four Dravidian tracks (after [Tamil](../tamil/README.md) the anchor,
[Kannada](../kannada/README.md), and [Telugu](../telugu/README.md)), built the
same way as: one word per lesson, taken apart and traced to its root; the
pieces taught before the whole; and a book you can read straight through.

## What's different about the Malayalam track

- **The script is taught inside the word lessons — no reading course.** Each
  word lesson has a *"The letters in this word"* section introducing exactly
  the letters that word needs (നമസ്കാരം brings the chandrakkala, the സ്ക
  conjunct, and the anusvāram ം). A reader who already reads Malayalam skims
  those notes.
- **Malayalam as Tamil's closest sister — with a Sanskrit overlay.** The two
  split only ~1000 years ago, so the everyday core is strikingly Tamil-like:
  *nandi* (= Tamil *naṉṟi*), *illa* (= Tamil *illai*), *śari* (= *sari*), and
  the *pōyi varām* farewell (≈ Tamil *pōy varugiṟēṉ*). Yet a vast Sanskrit
  layer (the *Maṇipravāḷam* tradition) gives it a Sanskrit formal greeting
  (*namaskāram*) and the largest alphabet of the family. Each lesson carries an
  **"Across the family"** cognate box, every form supplied so nothing is
  assumed — completing the four-way pattern-matching across the Dravidian
  scripts.
- **Grammar introduced inline**: yes/no as demonstratives (*athe*/*alla*) at
  *athe*, negation-by-verb (the shared *il-*) at *illa*.

## Progress

- **Chapter 1 — Greetings** ([`lessons/ML-C01-*`](./lessons/)): namaskāram →
  nandi → athe → illa → śari → practice (with the *pōyi varām* farewell).
  Malayalam script taught inline; Dravidian cognates traced. In the book.
- **Chapter 2 — Introducing Yourself** ([`lessons/ML-C02-*`](./lessons/)):
  peru, enṟe, **āṇŭ** ("is" — the copula), **enṟe pēru … āṇŭ** ("my name is"),
  nī/niṅṅaḷ, entŭ, **ninṟe pēru entāṇŭ?** ("what's your name?"), santōṣam,
  practice. The Malayalam standout: it *has* a copula *āṇŭ*, unlike its
  zero-copula Dravidian sisters. In the book.
- **Chapter 3 — How Are You** ([`lessons/ML-C03-*`](./lessons/)): eṅṅane,
  **sukhamāṇō?** ("are you well?"), ñān, sukham, sāramilla, practice. The
  copula *āṇŭ* + question *-ō*; Sanskrit *sukha*/*sāraṁ* on native grammar. In
  the book.
- **Chapter 4 — Farewells** ([`lessons/ML-C04-*`](./lessons/)): pōkuka/varika,
  **pōyi varāṁ** ("I'll go and come back"), nāḷe kāṇāṁ, vīṇḍuṁ kāṇāṁ, practice.
  The Dravidian promise-of-return goodbye. In the book.
- **Chapter 5 — First Verbs** ([`lessons/ML-C05-*`](./lessons/)): saṁsārikkuka,
  **ñān malayāḷaṁ saṁsārikkunnu**, tāmasikkuka, jōli ceyyuka, practice. The
  *-unnu* present — the verb never changes for person. In the book.
- **Chapters 6–9 — Case, counting, and courtesy**
  ([`lessons/ML-C0{6,7,8,9}-*`](./lessons/)): the dative **-ഇക്ക്/-ഇന്**, the
  dative-subject sentence **എനിക്ക് മലയാളം അറിയാം**, numbers one through ten,
  **ദയവായി**, and **ക്ഷമിക്കണം**. In the book.
- **Chapters 10–22 — Calendar and everyday domains**
  ([`lessons/ML-C{10..22}-*`](./lessons/)): days, colours, family, body,
  seasons, food, Malayalam's solar months, clock time, age, numbers 11–20,
  weather, and animals. In the book.
- **Chapters 23–31 — Dayparts and greetings**
  ([`lessons/ML-C{23..31}-*`](./lessons/)): native and Sanskrit day/night
  words, morning, evening, afternoon, and their register-aware greetings.
- **Chapter 32 — The Core Verbs** ([`lessons/ML-C32-*`](./lessons/)): uṇṭŭ /
  irikkuka (be), pōkuka (go), varuka (come), tinnuka (eat), kāṇuka (see),
  aṟiyuka (know) — the track's first A2 chapter, and the one that names
  Malayalam's signature. A Malayalam verb is **stem plus one ending, with no
  person slot at all**: *ñān pōkunnu*, *nī pōkunnu*, *avan pōkunnu*, one form
  for everybody, where Tamil needs *pōgiṟēṉ / pōgiṟāy / pōgiṟāṉ*. The room that
  frees up goes to **mood** (*kāṇāṁ*, *kāṇaṇaṁ*, *kāṇarutŭ*), the irregularity
  budget is spent entirely on the past (*vannu*, *kaṇṭu*), and the chapter
  closes on **അറിയുക**, which will not take a subject at all. In the book, and
  drivable end to end.

All thirty-nine later lessons remain below five effective minutes.

## Book / fonts

---

## For contributors

Everything below this line is about how the track is built and checked. It is
here for people working on the curriculum; nothing in it is needed to learn the
language.

## Chapter capabilities

[`chapters.json`](./chapters.json) is the track's
[`HL05`](../../../specs/HL05-chapter-capability-and-step-by-step-shape.md)
capability ledger. Each entry says, in the reader's own first-person words, what
finishing that chapter lets them do, and names the lesson that proves it:

```json
{
  "chapter": 19,
  "title": "Asking Someone's Age",
  "canDo": "I can ask how old someone is in Malayalam and answer with a dative subject.",
  "payoff": { "lesson": "ML-C19-vayassu", "kind": "dialogue", "assesses": ["…"] }
}
```

The file is **authored intent**, not a derived cache — no validator may rewrite
it, and it is derived from the lessons themselves rather than from `roadmap.md`,
which still lags Chapters 6–32. Chapters 6–32 are covered. Chapters 1–5 are
deliberately left out: their recap lessons are still schema v1 with no declared
knowledge atoms, so a payoff there could only be invented. That absence is
honest, measurable debt and is reported as such.

## Files

- [`lessons/`](./lessons/) · [`chapters.json`](./chapters.json)
  · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `ML-C01-namaskaram`); order lives in the book and
`session-map.md`.
