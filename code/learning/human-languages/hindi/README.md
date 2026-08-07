# Hindi

The fifth track of the [Human Languages](../README.md) curriculum, built the
same way as: one word per lesson, taken apart and traced to its root; the
pieces taught before the whole; and a book you can read straight through.

## What's different about the Hindi track

Two things shape this track.

- **The script is taught inside the word lessons — no reading course.** Written
  for someone who may not read a single Devanagari letter, each word lesson has
  a *"The letters in this word"* section that introduces exactly the letters
  that word needs (नमस्ते brings न म स त and the *e*-mātrā and a conjunct), so
  you learn to read the word *and* what it means in the same few minutes. A
  reader who already knows Devanagari simply skims those notes. 
- **Hindi's double inheritance is the recurring thread.** Its core vocabulary
  is **Sanskrit** (*namaste*, *dhanyavād* — roots *nam* "to bow," *dhanya*
  "worthy"), but centuries of Persian court rule layered on a second,
  **Perso-Arabic** vocabulary — so "thanks" is also *shukriyā* (the same
  Semitic root **sh-k-r** as Arabic *shukran*) and "farewell" is *alvidā*
  (carrying the Arabic article *al-*). Grounded against English + Arabic, the
  track keeps both streams in view.

## Progress

- **Chapter 1 — Greetings** ([`lessons/HI-C01-*`](./lessons/)): namaste →
  namaskār → dhanyavād → shukriyā → alvidā → practice. Devanagari taught
  inline (inherent *a*, mātrā vowel signs, halant + conjuncts, independent
  vowels) and the Sanskrit vs. Perso-Arabic heritage introduced through the
  words themselves. In the book.
- **Chapter 2 — Introducing Yourself** ([`lessons/HI-C02-*`](./lessons/)): nām,
  merā, hai, **merā nām … hai** ("my name is"), āp/tum, kyā, **āpkā nām kyā
  hai?** ("what's your name?"), khushī, practice. Every atom traced (nām ←
  *nāman* → *name*; hai ← *asti* → *is*); SOV order; the three-level "you". In
  the book.
- **Chapter 3 — How Are You** ([`lessons/HI-C03-*`](./lessons/)): kaise, **āp
  kaise haiṁ?**, maiṁ, hūṁ, ṭhīk, āpkā svāgat hai, practice. The copula trio
  hūṁ/hai/haiṁ (← *asmi/asti* → *am/is*); respect-as-plural. In the book.
- **Chapter 4 — Farewells** ([`lessons/HI-C04-*`](./lessons/)): phir, milenge,
  **phir milenge**, kal milte haiṁ (*kal* = tomorrow *and* yesterday), chaltā
  hūṁ, practice. In the book.
- **Chapter 5 — First Verbs** ([`lessons/HI-C05-*`](./lessons/)): bolnā, **maiṁ
  hindī boltā hūṁ**, rahnā, karnā (← √kṛ), practice. The present habitual and
  gender agreement. In the book.
- **Chapters 6–33 — Shared-spine expansion**
  ([`lessons/HI-C06-*`](./lessons/)): forty prerequisite-ordered lessons add
  numbers, yes/no and polite repair, days and time, colours, family, body,
  seasons, food, months, age, weather, animals, and progressively finer
  morning/evening/afternoon register.
- **Chapter 34 — Four Verbs of the Mind** ([`lessons/HI-C34-*`](./lessons/)):
  सोचना (think — from *śocati*, "to grieve"), समझना (understand — *sam-* +
  *budh-*, the root of **Buddha**), पढ़ना (read *and* study — *paṭhati*, "to
  recite aloud"), लिखना (write — *likhati*, "to scratch," the picture Latin,
  Greek and Old English each reached on their own). In the book.
- **Chapter 35 — Taking, Asking, Helping — and Liking**
  ([`lessons/HI-C35-*`](./lessons/)): लेना (take — *labhate* worn down, with
  लाभ preserving the lost *-bh-*), पूछना (ask — the same inherited verb as
  English **pray**), मदद करना (help — an Arabic noun plus native करना, Hindi's
  conjunct-verb engine), and पसंद, which is **not a verb at all**: *mujhe roṭī
  pasand hai* is "to me, bread is pleasing." In the book.
- **Writing companions W01–W05** ([`lessons/HI-W*`](./lessons/)): eleven short
  steps introduce the headline, letter bodies, inherent vowel, mātrās,
  preposed short **ि**, spineless letters, virama, conjuncts, and finally whole
  words. They remain inline in the opening chapters rather than becoming a
  prerequisite alphabet drill.

---

## For contributors

Everything below this line is about how the track is built and checked. It is
here for people working on the curriculum; nothing in it is needed to learn the
language.

## Chapter capabilities (`chapters.json`)

[`chapters.json`](./chapters.json) is the track's
[`HL05`](../../../specs/HL05-chapter-capability-and-step-by-step-shape.md)
capability ledger: for each chapter, one first-person `canDo` promise, the
shared-spine nodes it realises, and the **payoff** lesson that proves the
promise, with the exact knowledge atoms that lesson practises. It is authored
intent, not a derived cache — no validator may rewrite it.

Thirty-two of the thirty-five chapters are authored. Three are honestly missing:

- **Chapters 3, 4, and 5 have no entry.** Every lesson in them is still schema
  v1 with no `practises.knowledge`, so a payoff there could not name a single
  real atom. The absence is measurable debt; a placeholder would hide it.
- **Chapters 1 and 2 pay off in a writing lesson.** Their `HI-C0*-practice`
  lessons are also schema v1, so the payoff falls back to the last lesson by
  `sequence` — `HI-W02-ka-ta-mouth-order` and `HI-W05-write-namaste`. Both are
  recorded as `kind: task`, because forming क, त, and नमस्ते by hand is a
  writing task and calling it a dialogue would be a lie about the chapter.

Four chapters (1, 2, 6, and 32) currently assess less than half the atoms their
chapter introduces and will be reported under the 0.5 representativeness
threshold once the HL05 gates land. The cure is a real terminal consolidation
lesson in each, not a broader claim here. Chapters 34 and 35 fire **no** gate
findings: each payoff closes over its own chapter (5 of 8 atoms, and 6 of 9)
and every atom it names was taught by that chapter or earlier.

## Book / fonts

The 35-chapter book compiles with XeLaTeX using **vendored** Noto Sans
Devanagari, Noto Naskh Arabic, and Noto Sans Cyrillic fonts (`../../_fonts/`),
loaded by relative path — so it builds identically locally and in CI, with no
system-font dependency. Chapters 6–35 are generated from canonical lesson ASTs
and checked against Language Ladder source hashes. A forced 124-page build
reports **zero** missing characters and zero errors; the one overfull and one
underfull box both pre-date chapters 34–35. `latexmk -xelatex book.tex`.

## Files

- [`lessons/`](./lessons/) · [`chapters.json`](./chapters.json)
  · [`curriculum.json`](./curriculum.json)
  · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `HI-C01-namaste`); order lives in the book and
`session-map.md`.
