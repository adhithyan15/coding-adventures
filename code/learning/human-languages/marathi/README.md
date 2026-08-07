# Marathi

The twelfth track of the [Human Languages](../README.md) curriculum, built the
same way as: one word per lesson, taken apart and traced to its root; the
pieces taught before the whole; and a book you can read straight through.

## What's different about the Marathi track

- **Devanagari, taught inline — but Marathi, not Hindi.** Marathi is written in
  the same script as Hindi/Sanskrit and reuses the vendored Devanagari font,
  but it is its own **Indo-Aryan** language (of Maharashtra). The recurring
  thread is what makes it distinct: it prefers **namaskār** as the greeting,
  keeps **three** genders (Hindi has two), marks **gender on the verb** even in
  the present (*yeto* m. / *yete* f.), and has an **extra letter ळ** (retroflex
  *ḷ*) shared with the Dravidian south. Each word lesson has a *"The letters in
  this word"* section; a reader who knows Devanagari skims it.
- **Grounded against English + Sanskrit**, with the wider Indo-European family
  drawn in where it reaches (*nāhī* ← PIE *\*ne*, English *no*).

## Progress

- **Chapter 1 — Greetings** ([`lessons/MR-C01-*`](./lessons/)): namaskār,
  dhanyavād, ho, nāhī, baraṁ, yeto/yete (gendered farewell), practice.
- **Chapter 2 — Introducing Yourself** ([`lessons/MR-C02-*`](./lessons/)): nāv,
  mājhaṁ, āhe, "my name is…", tū/tumhī, kāy, "what's your name?", ānand.
- **Chapter 3 — How Are You** ([`lessons/MR-C03-*`](./lessons/)): kasā, "tumhī
  kase āhāt?", mī, "mī barā āhe", kāhī harkat nāhī.
- **Chapter 4 — Farewells** ([`lessons/MR-C04-*`](./lessons/)): punhā, bheṭū,
  "punhā bheṭū", "udyā bheṭū", kāḷjī ghyā.
- **Chapter 5 — The First Verbs** ([`lessons/MR-C05-*`](./lessons/)): bolṇe,
  "mī marāṭhī bolto", rāhṇe, kām karṇe.
- **Chapter 6 — Numbers 1–5** ([`lessons/MR-C06-*`](./lessons/)): a short
  counting lesson followed by a prerequisite-ordered etymology lesson on why
  *don* copied *tīn*, why Hindi retains *pāṁch*'s nasal, and why written *chār*
  sounds nearer *tsār* in Marathi.

Chapters 1–6 are in the book.

---

## For contributors

Everything below this line is about how the track is built and checked. It is
here for people working on the curriculum; nothing in it is needed to learn the
language.

## What each chapter lets you do

[`chapters.json`](./chapters.json) is the HL05 capability ledger: per chapter, one
first-person can-do sentence and the lesson that pays it off.

- **Chapter 6** — *"I can count from one to five in Marathi, say चार as tsār
  rather than chār, and tell which of Marathi's differences from Hindi is an
  innovation and which is Hindi holding on to something older."* Payoff:
  [`MR-C06-number-differences`](./lessons/MR-C06-number-differences.md), a task —
  **दोन**'s borrowed *-n*, **पाच**'s missing nasal, and the *ts* hiding behind an
  unchanged spelling.

Chapters 1–5 are **not in the ledger yet**, and that gap is deliberate. They are
still schema v1, so their lessons declare no knowledge atoms and no payoff there
could honestly claim to assess anything. A placeholder would hide debt the HL05
gap report is meant to surface; the entries land as those chapters migrate.

## Book / fonts

Compiles with XeLaTeX using the **vendored** Noto Sans Devanagari font
(`../../_fonts/`) — the same font as the Hindi track. `latexmk -xelatex book.tex`.
The six-chapter build is warning-clean, and its PDF outline preserves readable
Devanagari while generated non-Latin sections use bookmark-safe romanization.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `MR-C01-namaskar`); order lives in the book and
`session-map.md`.
