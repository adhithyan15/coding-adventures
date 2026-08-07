# Kannada

The seventh track of the [Human Languages](../README.md) curriculum, the
second of the four Dravidian tracks (after [Tamil](../tamil/README.md), the
anchor), built the same way as:
one word per lesson, taken apart and traced to its root; the
pieces taught before the whole; and a book you can read straight through.

## What's different about the Kannada track

- **The script is taught inside the word lessons — no reading course.** Written
  for someone who may not read a single Kannada letter, each word lesson has a
  *"The letters in this word"* section introducing exactly the letters that
  word needs (ನಮಸ್ಕಾರ brings ನ ಮ ಸ ಕ ರ, the long-ā vowel sign, and the ಸ್ಕ
  *ottakṣara* conjunct). A reader who already reads Kannada skims those notes.
- **Kannada as the Sanskrit-borrowing Dravidian.** The recurring thread is the
  contrast with Tamil: Kannada borrowed *freely* from Sanskrit (*namaskāra*,
  *dhanyavāda*) yet keeps native Dravidian words for the everyday grammar
  (*haudu*, *illa* ≈ Tamil *illai*, *sari* ≈ Tamil *sari*). Each lesson carries
  an **"Across the family"** cognate box (English / Sanskrit / Hindi / Tamil /
  the Dravidian sisters), every form supplied so no prior knowledge is assumed
  — built for pattern-matching across the four Dravidian scripts.
- **Grammar introduced inline**: verb-echo "yes" at *haudu*, negation-by-verb
  at *illa* (the shared Dravidian *il-*), one-word-two-scripts at *sari*.

## Progress

- **Chapter 1 — Greetings** ([`lessons/KA-C01-*`](./lessons/)): namaskāra →
  dhanyavāda → haudu → illa → sari → practice (with the *hōgi baruttēne*
  farewell). Kannada script taught inline; Dravidian cognates traced. In the
  book.
- **Chapter 2 — Introducing Yourself** ([`lessons/KA-C02-*`](./lessons/)):
  hesaru, nanna, **nanna hesaru** ("my name is," zero copula), nīnu/nīvu, ēnu,
  **nimma hesaru ēnu?** ("what's your name?"), santōṣa, practice. Every atom
  traced (*hesaru* ← *\*pesar*, p→h; *santōṣa* Sanskrit vs. Tamil *magiḻcci*).
  In the book.
- **Chapter 3 — How Are You** ([`lessons/KA-C03-*`](./lessons/)): hēge, **nīvu
  hēgiddīrā?**, nānu, cennāgi, paravāgilla, practice. The verb *iru* ("to be,"
  same as Tamil); the Dravidian *illa*. In the book.
- **Chapter 4 — Farewells** ([`lessons/KA-C04-*`](./lessons/)): hōgu/bā, **hōgi
  baruttēne** ("I'll go and come back"), nāḷe sigōṇa, matte sigōṇa, practice.
  The Dravidian promise-of-return goodbye. In the book.
- **Chapter 5 — First Verbs** ([`lessons/KA-C05-*`](./lessons/)): mātanāḍu,
  **nānu kannaḍa mātanāḍuttēne**, iru, kelasa māḍu, practice. Stem + tense +
  person; no 1st-person gender. In the book.
- **Chapters 6–9 — Case, counting, and courtesy**
  ([`lessons/KA-C0{6,7,8,9}-*`](./lessons/)): the dative **-ಗೆ**, visible
  suffix stacking, **ನನಗೆ ಕನ್ನಡ ಗೊತ್ತು**, numbers one through ten,
  **ದಯವಿಟ್ಟು**, and **ಕ್ಷಮಿಸಿ**. In the book.
- **Chapters 10–22 — Calendar and everyday domains**
  ([`lessons/KA-C{10..22}-*`](./lessons/)): days, colours, family, body,
  seasons, food, months, clock time, age, numbers 11–20, weather, and animals.
  In the book.
- **Chapters 23–31 — Dayparts and greetings**
  ([`lessons/KA-C{23..31}-*`](./lessons/)): day, night, morning, evening,
  afternoon, and their register-aware greetings.

All thirty later lessons remain below five effective minutes.

---

## For contributors

Everything below this line is about how the track is built and checked. It is
here for people working on the curriculum; nothing in it is needed to learn the
language.

## Chapter capabilities

[`chapters.json`](./chapters.json) is the track's
[`HL05`](../../../specs/HL05-chapter-capability-and-step-by-step-shape.md)
capability ledger: for each chapter, one first-person `canDo` ("I can tell
someone the time on the hour in Kannada"), the shared spine nodes it realises,
and the `payoff` lesson that proves the claim, with the exact knowledge atoms
that payoff exercises.

Chapters **6–31** are authored — twenty-six entries. Chapters **1–5 are absent
on purpose**: their lessons are still schema v1 with no `practises.knowledge`
and no `core/book-generation.json` target, so a payoff for them could only be
invented. That absence is measurable debt, not a placeholder.

Because no chapter after 5 has a terminal `practice` lesson, each payoff is the
chapter's last lesson by `sequence` — in practice the lesson whose Guided
Practice block recombines everything the chapter taught.

## Book / fonts

The book compiles with XeLaTeX using the **vendored** Noto Sans Kannada font
(`../../_fonts/`), loaded by relative path — so it builds identically locally
and in CI, no system-font dependency. `latexmk -xelatex book.tex`.

## Files

- [`lessons/`](./lessons/) · [`chapters.json`](./chapters.json)
  · [`curriculum.json`](./curriculum.json)
  · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `KA-C01-namaskara`); order lives in the book and
`session-map.md`.
