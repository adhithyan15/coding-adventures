# Bengali

A track of the [Human Languages](../README.md) curriculum, built the same way
as: one word per lesson, taken apart and traced to its root; the pieces taught
before the whole; and a book you can read straight through.

## What's different about the Bengali track

- **Bengali script, taught inline.** Bengali (*Bāṅlā*) is **Indo-Aryan** (like
  Hindi, Marathi, Punjabi), written in its own script — sister to Devanagari,
  sharing the hanging top line but with rounder shapes. A vendored Noto Sans
  Bengali font renders it; each word lesson has a *"The letters in this word"*
  section, and a reader who already reads Bengali simply skims it. No reading
  course.
- **One sound-shift as the spine.** The recurring thread is Bengali's
  **fingerprint**: the inherent vowel is **ô** (not "a"), *s* tilts to **sh**,
  and *v* collapses to **b** — so Sanskrit *namaskāra* is spoken **nômoshkar**
  and *dhanyavāda* becomes **dhônyobad**. Learn the shift and every
  Sanskrit-derived word becomes legible.
- **Grounded against English + Sanskrit**, with the wider Indo-European family
  drawn in where it reaches (*nā* ← PIE *ne*, English *no*).

## Progress

- **Chapter 1 — Greetings** ([`lessons/BN-C01-*`](./lessons/)): nômoshkar,
  dhônyobad, hyã/nā, āchchhā, āshi (the "I'll come again" goodbye), practice.
  In the book.
- **Chapter 2 — Introducing Yourself** ([`lessons/BN-C02-*`](./lessons/)): nām,
  āmār, **āmār nām …** ("my name is," zero copula), tumi/āpni (+ tui), ki,
  **tomār nām ki?**, ālāp kore bhālo lāglo, practice. The zero copula; no
  gender. In the book.
- **Chapter 3 — How Are You** ([`lessons/BN-C03-*`](./lessons/)): kemon, **tumi
  kemon āchho?**, āmi (← *asmi* → *am*), bhālo, kono bæpār nā, practice. The
  verb *āchhā* returns for state. In the book.
- **Chapter 4 — Farewells** ([`lessons/BN-C04-*`](./lessons/)): ābār, dækhā
  hôbe, **ābār dækhā hôbe**, kāl dækhā hôbe, practice. The impersonal future.
  In the book.
- **Chapter 5 — First Verbs** ([`lessons/BN-C05-*`](./lessons/)): bôlā, **āmi
  bānglā bôli**, thākā (← *sthā* → *stand/stay*), kāj kôrā, practice. The verb
  never changes for gender. In the book.
- **Chapter 6 — Numbers 1–5** ([`lessons/BN-C06-*`](./lessons/)): *ek, dui,
  tin, chār, pā̃ch*, the chandrabindu, and the conservative vowel in *dui*. In
  the book.
- **Chapter 7 — The Core Verbs** ([`lessons/BN-C07-*`](./lessons/)): hôwā (two
  be-verbs, and আছ- has no future), jāwā (respect rides on the ending), āsā (no
  grammatical gender, anywhere), khāwā (Bengali eats its drinks), dækhā (vowel
  harmony under a fixed spelling), jānā (জানা for facts, চেনা for people). The
  first six **canonical** verb concepts the track has ever realized. In the
  book, and fully drivable.

---

## For contributors

Everything below this line is about how the track is built and checked. It is
here for people working on the curriculum; nothing in it is needed to learn the
language.

## What each chapter lets you do

[`chapters.json`](./chapters.json) is the HL05 capability ledger: per chapter, one
first-person can-do sentence and the lesson that pays it off.

- **Chapter 6** — *"I can count from one to five in Bengali script and say what
  দুই kept that Hindi and Marathi flattened away."* Payoff:
  [`BN-C06-numbers-1-5`](./lessons/BN-C06-numbers-1-5.md), a spoken production —
  count the five, name the chandrabindu on **পাঁচ**, place *dui* against *do* and
  *don*.
- **Chapter 7** — *"I can say what I do with six everyday Bengali verbs, pick
  the ending that matches তুই, তুমি or আপনি, and say why a Bengali verb never
  changes for gender and why আছে has to hand the future to হবে."* Payoff:
  [`BN-C07-jana`](./lessons/BN-C07-jana.md), a spoken production covering 8 of
  the chapter's 12 introduced atoms.

Chapters 1–5 are **not in the ledger yet**, and that gap is deliberate. They are
still schema v1, so their lessons declare no knowledge atoms and no payoff there
could honestly claim to assess anything. A placeholder would hide debt the HL05
gap report is meant to surface; the entries land as those chapters migrate.

## Book / fonts

Compiles with XeLaTeX using the **vendored** Noto Sans Bengali font
(`../../_fonts/NotoSansBengali-Static.ttf`). `latexmk -xelatex book.tex`.
Chapters 6 and 7 are generated from the canonical lessons; their Bengali-script
runs use that font while their section bookmarks use authored romanization.

The forced seven-chapter build is warning-free: main-font punctuation,
chapter-qualified recap anchors, bookmark-safe Bengali, natural page bottoms,
explicit static-font shapes, and a breakable long title keep the downloadable
PDF and its outline clean. Two details serve the generated chapters: the
`grammarlens` box takes an optional title, because the generator passes each
lesson's own "Grammar Lens: …" heading through, and `ǵ`/`ḱ` are composed with
`\newunicodechar` since the main font has no precomposed glyph for the
reconstructed PIE palatals.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `BN-C01-nomoshkar`); order lives in the book and
`session-map.md`.
