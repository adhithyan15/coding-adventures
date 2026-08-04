# Marathi

The twelfth track of the [Human Languages](../README.md) curriculum, on the same
[`HL00`](../../../specs/HL00-human-language-curriculum-framework.md) framework:
one word per lesson, slug ids, atom-first assembly, derivations shown, LaTeX
book.

## What's different about the Marathi track

- **Devanagari, taught inline — but Marathi, not Hindi.** Marathi is written in
  the same script as Hindi/Sanskrit and reuses the vendored Devanagari font, but
  it is its own **Indo-Aryan** language (of Maharashtra). The recurring thread is
  what makes it distinct: it prefers **namaskār** as the greeting, keeps **three**
  genders (Hindi has two), marks **gender on the verb** even in the present
  (*yeto* m. / *yete* f.), and has an **extra letter ळ** (retroflex *ḷ*) shared
  with the Dravidian south. Each word lesson has a *"The letters in this word"*
  section; a reader who knows Devanagari skims it.
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
- **Chapter 5 — The First Verbs** ([`lessons/MR-C05-*`](./lessons/)): bolṇe, "mī
  marāṭhī bolto", rāhṇe, kām karṇe.
- **Chapter 6 — Numbers 1–5** ([`lessons/MR-C06-*`](./lessons/)): a short
  counting lesson followed by a prerequisite-ordered etymology lesson on why
  *don* copied *tīn*, why Hindi retains *pāṁch*'s nasal, and why written *chār*
  sounds nearer *tsār* in Marathi.

Chapters 1–6 are in the book. Chapter 6 is generated from the same canonical
schema-v2 lesson AST and source hashes that Language Ladder loads, while the
first five chapters retain their authored long-form narrative during migration.

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
